use crate::core::types::{
    Cause, CauseKind, DependencyEdgeView, DependencyRelation, DeterministicActionClass,
    DeterministicConvergenceRecord, DeterministicReconciliationPlan, DiffItem, ManagedObjectKind,
    ManagedObjectRef, PlanEntry, PlanEntryAction, PlanOutputView, PlanSummaryView, QuadletType,
    ReconcileRun, ReconciliationPlan, RevisionContext, RollbackTargetCandidate,
    SemanticDiffKind, SemanticDiffView, VerificationResult,
};
use crate::core::planner::{
    dependency_edges_for_object, dependent_refs, direct_prerequisite_refs, managed_object_ref,
    object_kind_by_id,
};
use crate::core::validation::validate_plan_output_view;
use std::collections::{BTreeMap, BTreeSet};

pub fn format_plan_report(plan: &ReconciliationPlan, diffs: &[DiffItem]) -> String {
    let mut output = String::new();
    output.push_str(&format!("plan with {} actions\n", plan.actions.len()));
    output.push_str(&format!("diffs {}\n", diffs.len()));
    let mut type_by_name = std::collections::HashMap::new();
    for diff in diffs {
        let quadlet_type = diff
            .desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|w| w.quadlet_type.clone());
        type_by_name.insert(diff.name.clone(), quadlet_type.clone());
        let quadlet_label = quadlet_type_label(quadlet_type);
        output.push_str(&format!(
            "- {:?}: {} [{}]\n",
            diff.kind, diff.name, quadlet_label
        ));
    }
    output.push_str("actions\n");
    for action in &plan.actions {
        let quadlet_type = type_by_name.get(&action.target).cloned().flatten();
        let action_label = action_label(&action.action_type, quadlet_type.as_ref());
        output.push_str(&format!("- {}: {}\n", action_label, action.target));
    }
    output
}

pub fn append_provenance_report(base: &str, contents: Option<&str>) -> String {
    match contents {
        Some(contents) => format!("{base}\n{}", crate::cli::status::format_status_text(contents)),
        None => base.to_string(),
    }
}

pub fn format_apply_report_json(
    run: &ReconcileRun,
    verification_results: &[VerificationResult],
) -> String {
    serde_json::json!({
        "run_id": run.run_id,
        "status": run_status_label(&run.status),
        "summary": run.summary,
        "verification_results": verification_results.iter().map(|result| serde_json::json!({
            "target": result.target,
            "status": verification_status_label(&result.status),
            "details": result.details,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

pub fn format_convergence_report_json(
    run: &ReconcileRun,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
) -> String {
    let base = serde_json::json!({
        "run_id": run.run_id,
        "status": run_status_label(&run.status),
        "summary": run.summary,
        "verification_results": verification_results.iter().map(|result| serde_json::json!({
            "target": result.target,
            "status": verification_status_label(&result.status),
            "details": result.details,
        })).collect::<Vec<_>>(),
        "convergence": convergence.map(|record| serde_json::json!({
            "desired_revision_id": record.desired_revision_id,
            "scope_id": record.scope_id,
            "status": convergence_status_label(&record.status),
            "attempt_count": record.attempt_count,
            "affected_objects": record.affected_objects,
            "completed_actions": record.completed_actions,
            "failed_actions": record.failed_actions,
            "can_continue": record.can_continue,
        })),
    });
    base.to_string()
}

pub fn build_revision_context(plan: &DeterministicReconciliationPlan) -> RevisionContext {
    RevisionContext {
        target_revision: plan
            .desired_revision_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        last_applied_revision: plan.baseline_revision_id.clone(),
        change_revision: plan.desired_revision_id.clone(),
    }
}

fn inferred_kind_for_missing_dependency(object_id: &str) -> ManagedObjectKind {
    if object_id.ends_with(".mount") {
        ManagedObjectKind::Mount
    } else if object_id.ends_with(".automount") {
        ManagedObjectKind::Automount
    } else if object_id.starts_with("config:") {
        ManagedObjectKind::RenderedArtifact
    } else if object_id.ends_with(".network")
        || object_id.ends_with(".container")
        || object_id.ends_with(".volume")
        || object_id.ends_with(".socket")
    {
        ManagedObjectKind::QuadletResource
    } else if object_id.ends_with(".service") {
        ManagedObjectKind::GeneratedUnit
    } else {
        ManagedObjectKind::GeneratedUnit
    }
}

fn inferred_managed_object_ref(object_id: &str) -> ManagedObjectRef {
    managed_object_ref(object_id, &inferred_kind_for_missing_dependency(object_id))
}

fn parse_diff_triplet(details: &str) -> Option<(String, String, String)> {
    let desired_prefix = "desired=";
    let actual_sep = " actual=";
    let applied_sep = " applied=";
    let desired = details.strip_prefix(desired_prefix)?;
    let actual_idx = desired.find(actual_sep)?;
    let (desired_value, remainder) = desired.split_at(actual_idx);
    let remainder = remainder.strip_prefix(actual_sep)?;
    let applied_idx = remainder.find(applied_sep)?;
    let (actual_value, applied_value) = remainder.split_at(applied_idx);
    let applied_value = applied_value.strip_prefix(applied_sep)?;
    Some((
        desired_value.to_string(),
        actual_value.to_string(),
        applied_value.to_string(),
    ))
}

fn diff_summary(action: &crate::core::types::DeterministicPlannedAction) -> String {
    let changed_fields = significant_diff_fields(action);
    if changed_fields == vec!["contents".to_string()] {
        "content".to_string()
    } else if changed_fields.len() == 1 {
        if changed_fields[0] == "contents" {
            "content".to_string()
        } else {
            changed_fields[0].clone()
        }
    } else if changed_fields.is_empty() && action.semantic_diff.contains_key("contents") {
        "content".to_string()
    } else if changed_fields.is_empty() {
        "definition".to_string()
    } else {
        format!("{} fields changed", changed_fields.len())
    }
}

fn significant_diff_fields(
    action: &crate::core::types::DeterministicPlannedAction,
) -> Vec<String> {
    let mut fields = action
        .semantic_diff
        .keys()
        .filter(|field| {
            !matches!(
                field.as_str(),
                "name" | "unit_name" | "quadlet_type" | "enabled_state" | "restart_policy"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if fields.is_empty() && action.semantic_diff.contains_key("contents") {
        fields.push("contents".to_string());
    }
    fields
}

fn line_based_unified_diff(details: &str) -> Option<String> {
    let (desired, actual, _) = parse_diff_triplet(details)?;
    let desired_lines = desired.lines().collect::<Vec<_>>();
    let actual_lines = actual.lines().collect::<Vec<_>>();
    let mut output = String::from("--- previous\n+++ desired\n");
    for line in actual_lines {
        if line != "<absent>" {
            output.push('-');
            output.push(' ');
            output.push_str(line);
            output.push('\n');
        }
    }
    for line in desired_lines {
        if line != "<absent>" {
            output.push('+');
            output.push(' ');
            output.push_str(line);
            output.push('\n');
        }
    }
    Some(output)
}

fn drift_details_for_object<'a>(
    plan: &'a DeterministicReconciliationPlan,
    object_id: &str,
) -> Option<&'a crate::core::types::StructuredDriftRecord> {
    plan.drift_records.iter().find(|record| record.object_id == object_id)
}

pub fn build_plan_causes(
    plan: &DeterministicReconciliationPlan,
    action: &crate::core::types::DeterministicPlannedAction,
) -> Vec<Cause> {
    let mut causes = Vec::new();
    if let Some(record) = drift_details_for_object(plan, &action.object_id) {
        let mut details = BTreeMap::new();
        details.insert(
            "comparison_basis".to_string(),
            record.comparison_basis.clone(),
        );
        details.insert("details".to_string(), record.details.clone());
        causes.push(Cause {
            kind: CauseKind::Drift,
            summary: action.reason.clone(),
            source_object: None,
            details: Some(details),
        });
    } else {
        let kind = match action.classification {
            DeterministicActionClass::Create
            | DeterministicActionClass::Update
            | DeterministicActionClass::Delete => CauseKind::DesiredChange,
            DeterministicActionClass::Replace => CauseKind::ReplacementRequired,
            DeterministicActionClass::Restart => CauseKind::DependencyChange,
            DeterministicActionClass::Blocked => CauseKind::BlockedPrerequisite,
            DeterministicActionClass::NoOp => CauseKind::NoChange,
        };
        if !matches!(action.classification, DeterministicActionClass::NoOp) {
            let source_object = if matches!(action.classification, DeterministicActionClass::Restart)
            {
                restart_trigger_object(plan, action)
            } else {
                None
            };
            causes.push(Cause {
                kind,
                summary: action.reason.clone(),
                source_object,
                details: None,
            });
        }
    }
    causes
}

fn restart_trigger_object(
    plan: &DeterministicReconciliationPlan,
    action: &crate::core::types::DeterministicPlannedAction,
) -> Option<ManagedObjectRef> {
    let object_kinds = object_kind_by_id(&plan.graph);
    action.dependency_context.iter().find_map(|dependency| {
        plan.actions
            .iter()
            .find(|candidate| candidate.object_id == *dependency)
            .and_then(|candidate| {
                matches!(
                    candidate.classification,
                    DeterministicActionClass::Create
                        | DeterministicActionClass::Update
                        | DeterministicActionClass::Replace
                        | DeterministicActionClass::Restart
                )
                .then(|| {
                    object_kinds
                        .get(dependency.as_str())
                        .map(|kind| managed_object_ref(dependency, kind))
                        .unwrap_or_else(|| inferred_managed_object_ref(dependency))
                })
            })
    })
}

pub fn build_dependency_edges(
    plan: &DeterministicReconciliationPlan,
    action: &crate::core::types::DeterministicPlannedAction,
) -> Vec<DependencyEdgeView> {
    let object_kinds = object_kind_by_id(&plan.graph);
    let direct_ids = direct_prerequisite_refs(&plan.graph, &action.object_id)
        .into_iter()
        .map(|object| object.name)
        .collect::<BTreeSet<_>>();

    let mut dependencies = dependency_edges_for_object(&plan.graph, &action.object_id);
    for dependency in &action.dependency_context {
        if !direct_ids.contains(dependency) {
            let object = object_kinds
                .get(dependency.as_str())
                .map(|kind| managed_object_ref(dependency, kind))
                .unwrap_or_else(|| inferred_managed_object_ref(dependency));
            dependencies.push(DependencyEdgeView {
                relation: DependencyRelation::Blocker,
                object,
            });
        }
    }
    dependencies.sort_by(|a, b| a.object.display_id.cmp(&b.object.display_id));
    dependencies
}

pub fn inspect_plan_dependencies(
    plan: &DeterministicReconciliationPlan,
    object_id: &str,
) -> Vec<DependencyEdgeView> {
    let mut edges = dependency_edges_for_object(&plan.graph, object_id);
    if let Some(action) = plan.actions.iter().find(|action| action.object_id == object_id) {
        let direct_ids = direct_prerequisite_refs(&plan.graph, object_id)
            .into_iter()
            .map(|object| object.name)
            .collect::<BTreeSet<_>>();
        let object_kinds = object_kind_by_id(&plan.graph);
        edges.extend(action.dependency_context.iter().filter_map(|dependency| {
            (!direct_ids.contains(dependency)).then(|| DependencyEdgeView {
                relation: DependencyRelation::Blocker,
                object: object_kinds
                    .get(dependency.as_str())
                    .map(|kind| managed_object_ref(dependency, kind))
                    .unwrap_or_else(|| inferred_managed_object_ref(dependency)),
            })
        }));
    }
    edges.extend(
        dependent_refs(&plan.graph, object_id)
            .into_iter()
            .map(|object| DependencyEdgeView {
                relation: DependencyRelation::Dependent,
                object,
            }),
    );
    edges.sort_by(|a, b| {
        (&a.object.display_id, dependency_relation_label(&a.relation))
            .cmp(&(&b.object.display_id, dependency_relation_label(&b.relation)))
    });
    edges
}

pub fn build_semantic_diff(
    action: &crate::core::types::DeterministicPlannedAction,
) -> Option<SemanticDiffView> {
    if action.semantic_diff.is_empty() {
        return None;
    }
    let summary = diff_summary(action);
    let kind = match action.classification {
        DeterministicActionClass::Create => SemanticDiffKind::Creation,
        DeterministicActionClass::Delete => SemanticDiffKind::Deletion,
        DeterministicActionClass::Replace => SemanticDiffKind::Replacement,
        _ if action.semantic_diff.contains_key("contents") => SemanticDiffKind::LineBased,
        _ => SemanticDiffKind::SemanticOnly,
    };
    Some(SemanticDiffView {
        kind,
        summary,
        unified_diff: action
            .semantic_diff
            .get("contents")
            .and_then(|details| line_based_unified_diff(details)),
        details: Some(action.semantic_diff.clone()),
    })
}

fn plan_entry_action(action: &DeterministicActionClass) -> PlanEntryAction {
    match action {
        DeterministicActionClass::Create => PlanEntryAction::Create,
        DeterministicActionClass::Update => PlanEntryAction::Update,
        DeterministicActionClass::Delete => PlanEntryAction::Delete,
        DeterministicActionClass::Replace => PlanEntryAction::Replace,
        DeterministicActionClass::Restart => PlanEntryAction::Restart,
        DeterministicActionClass::NoOp => PlanEntryAction::NoOp,
        DeterministicActionClass::Blocked => PlanEntryAction::Blocked,
    }
}

pub fn build_plan_output(plan: &DeterministicReconciliationPlan) -> PlanOutputView {
    let object_kinds = object_kind_by_id(&plan.graph);
    let entries = plan
        .actions
        .iter()
        .enumerate()
        .map(|(order_index, action)| {
            let object = object_kinds
                .get(action.object_id.as_str())
                .map(|kind| managed_object_ref(&action.object_id, kind))
                .unwrap_or_else(|| inferred_managed_object_ref(&action.object_id));
            let action_kind = plan_entry_action(&action.classification);
            let unchanged = matches!(action_kind, PlanEntryAction::NoOp);
            PlanEntry {
                object,
                action: action_kind,
                causes: build_plan_causes(plan, action),
                dependencies: build_dependency_edges(plan, action),
                order_index,
                diff: build_semantic_diff(action),
                unchanged: Some(unchanged),
                notes: None,
            }
        })
        .collect::<Vec<_>>();

    let summary = PlanSummaryView {
        changed_count: entries
            .iter()
            .filter(|entry| {
                !matches!(entry.action, PlanEntryAction::NoOp | PlanEntryAction::Blocked | PlanEntryAction::Skipped)
            })
            .count(),
        unchanged_count: entries
            .iter()
            .filter(|entry| matches!(entry.action, PlanEntryAction::NoOp))
            .count(),
        blocked_count: entries
            .iter()
            .filter(|entry| matches!(entry.action, PlanEntryAction::Blocked))
            .count(),
        skipped_count: entries
            .iter()
            .filter(|entry| matches!(entry.action, PlanEntryAction::Skipped))
            .count(),
        total_count: Some(entries.len()),
    };

    let view = PlanOutputView {
        view_kind: "plan".to_string(),
        revision_context: build_revision_context(plan),
        summary,
        entries,
    };
    validate_plan_output_view(&view).expect("plan output view must remain valid");
    view
}

pub fn format_deterministic_plan_report(plan: &DeterministicReconciliationPlan) -> String {
    format_deterministic_plan_report_with_options(plan, false)
}

pub fn format_deterministic_plan_report_with_options(
    plan: &DeterministicReconciliationPlan,
    verbose: bool,
) -> String {
    let view = build_plan_output(plan);
    let entry_by_name = view
        .entries
        .iter()
        .map(|entry| (entry.object.name.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let label_column = plan_label_column(plan, &view.entries, &entry_by_name);
    let header = format!(
        "Plan for host {} @ {}",
        display_scope_name(&plan.scope_id),
        plan_header_revision_context(
            view.revision_context.last_applied_revision.as_deref(),
            &view.revision_context.target_revision,
            !plan.drift_records.is_empty(),
        ),
    );
    let mut output = String::new();
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        header,
        color_reset(),
    ));
    output.push_str(&format!("{}\n", "─".repeat(header.chars().count())));
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::Create,
        "Create",
        verbose,
    );
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::Update,
        "Update",
        verbose,
    );
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::Restart,
        "Restart",
        verbose,
    );
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::Blocked,
        "Blocked",
        verbose,
    );
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::Delete,
        "Delete",
        verbose,
    );
    render_group(
        &mut output,
        plan,
        &view.entries,
        &entry_by_name,
        label_column,
        PlanEntryAction::NoOp,
        "Unchanged",
        verbose,
    );
    let summary_heading = "Summary";
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        summary_heading,
        color_reset(),
    ));
    output.push_str(&format!("{}\n", "─".repeat(summary_heading.chars().count())));
    output.push_str(&format!(
        "{}\n",
        plan_summary_line(
            &view.entries,
            view.summary.blocked_count,
            view.summary.unchanged_count,
            verbose,
        )
    ));
    output
}

fn significant_diff_fields_for_details(details: &BTreeMap<String, String>) -> Vec<String> {
    let mut fields = details
        .keys()
        .filter(|field| {
            !matches!(
                field.as_str(),
                "name" | "unit_name" | "quadlet_type" | "enabled_state" | "restart_policy"
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    fields.sort();
    fields
}

fn compact_diff_details(details: &str) -> String {
    if let Some((desired, actual, _)) = parse_diff_triplet(details) {
        format!("previous={} desired={}", compact_value(&actual), compact_value(&desired))
    } else {
        compact_value(details)
    }
}

fn compact_value(value: &str) -> String {
    let single_line = value.replace('\n', " ");
    if single_line.len() > 120 {
        format!("{}...", &single_line[..120])
    } else {
        single_line
    }
}

fn render_dependency_tree(
    plan: &DeterministicReconciliationPlan,
    object_id: &str,
    indent: &str,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
    label_column: usize,
) -> String {
    let mut output = String::new();
    let mut children = direct_prerequisite_refs(&plan.graph, object_id)
        .into_iter()
        .map(|object| (DependencyRelation::Prerequisite, object))
        .collect::<Vec<_>>();
    if let Some(action) = plan.actions.iter().find(|action| action.object_id == object_id) {
        let direct_ids = children
            .iter()
            .map(|(_, object)| object.name.clone())
            .collect::<BTreeSet<_>>();
        children.extend(action.dependency_context.iter().filter_map(|dependency| {
            (!direct_ids.contains(dependency)).then(|| {
                (DependencyRelation::Blocker, inferred_managed_object_ref(dependency))
            })
        }));
    }
    children.sort_by(|a, b| a.1.display_id.cmp(&b.1.display_id));
    for (index, (relation, object)) in children.iter().enumerate() {
        let branch = if index + 1 == children.len() { "└─" } else { "├─" };
        let entry = entry_by_name.get(object.name.as_str()).copied();
        output.push_str(&format_plan_label_line(
            &format!("{}{} ", indent, branch),
            entry
                .map(|entry| colored_marker(&entry.action))
                .or_else(|| Some(colored_marker_for_relation(relation))),
            &object.display_id,
            entry.map(short_label).unwrap_or_else(|| relation_label(relation)),
            label_column,
        ));
        if matches!(relation, DependencyRelation::Prerequisite) {
            output.push_str(&render_dependency_tree(
                plan,
                &object.name,
                &format!("{indent}   "),
                entry_by_name,
                label_column,
            ));
        }
    }
    output
}

fn short_label(entry: &PlanEntry) -> &'static str {
    match entry.action {
        PlanEntryAction::Create => "missing",
        PlanEntryAction::Update => "drift",
        PlanEntryAction::Replace => "drift",
        PlanEntryAction::Delete => "orphaned",
        PlanEntryAction::Restart => "dependency changed",
        PlanEntryAction::NoOp => "unchanged",
        PlanEntryAction::Blocked => "blocked",
        PlanEntryAction::Skipped => "skipped",
    }
}

fn full_cause_line(entry: &PlanEntry) -> Option<String> {
    let cause = entry.causes.first()?;
    match entry.action {
        PlanEntryAction::Create
        | PlanEntryAction::Update
        | PlanEntryAction::Replace
        | PlanEntryAction::Delete
        | PlanEntryAction::NoOp
        | PlanEntryAction::Skipped => None,
        PlanEntryAction::Restart => Some(
            cause
                .source_object
                .as_ref()
                .map(|object| format!("dependency changed: {}", object.display_id))
                .unwrap_or_else(|| cause.summary.clone()),
        ),
        PlanEntryAction::Blocked => {
            let summary = cause.summary.as_str();
            summary
                .eq_ignore_ascii_case(short_label(entry))
                .then_some(())
                .map(|_| None)
                .unwrap_or_else(|| Some(summary.to_string()))
        }
    }
}

fn relation_label(relation: &DependencyRelation) -> &'static str {
    match relation {
        DependencyRelation::Prerequisite => "required",
        DependencyRelation::Dependent => "dependent",
        DependencyRelation::Blocker => "blocked",
    }
}

fn colored_marker(action: &PlanEntryAction) -> String {
    let (color, symbol) = match action {
        PlanEntryAction::Create => (color_create(), "+"),
        PlanEntryAction::Update => (color_update(), "~"),
        PlanEntryAction::Replace => (color_update(), "~"),
        PlanEntryAction::Delete => (color_delete(), "-"),
        PlanEntryAction::Restart => (color_restart(), "↻"),
        PlanEntryAction::NoOp => (color_noop(), "·"),
        PlanEntryAction::Blocked => (color_blocked(), "!"),
        PlanEntryAction::Skipped => (color_noop(), "·"),
    };
    format!("{color}[{symbol}]{}", color_reset())
}

fn colored_marker_for_relation(relation: &DependencyRelation) -> String {
    match relation {
        DependencyRelation::Blocker => format!("{}[!]{}", color_blocked(), color_reset()),
        _ => format!("{}[·]{}", color_noop(), color_reset()),
    }
}

fn count_for_action(entries: &[PlanEntry], action: PlanEntryAction) -> usize {
    entries.iter().filter(|entry| entry.action == action).count()
}

fn summary_count_phrase(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

fn plan_header_revision_context(previous: Option<&str>, target: &str, has_drift: bool) -> String {
    match previous {
        None => format!("{} (first run)", short_revision(target)),
        Some(previous) if previous == target => {
            if has_drift {
                format!("{} (with drift)", short_revision(target))
            } else {
                short_revision(target).to_string()
            }
        }
        Some(previous) => {
            if has_drift {
                format!(
                    "{} → {} (with drift)",
                    short_revision(previous),
                    short_revision(target)
                )
            } else {
                format!("{} → {}", short_revision(previous), short_revision(target))
            }
        }
    }
}

fn plan_summary_line(
    entries: &[PlanEntry],
    blocked_count: usize,
    unchanged_count: usize,
    verbose: bool,
) -> String {
    let phrases = [
        summary_count_phrase(
            count_for_action(entries, PlanEntryAction::Create),
            "create",
            "creates",
        ),
        summary_count_phrase(
            count_for_action(entries, PlanEntryAction::Update),
            "update",
            "updates",
        ),
        summary_count_phrase(
            count_for_action(entries, PlanEntryAction::Restart),
            "restart",
            "restarts",
        ),
        summary_count_phrase(blocked_count, "blocked", "blocked"),
        summary_count_phrase(
            count_for_action(entries, PlanEntryAction::Delete),
            "delete",
            "deletes",
        ),
        summary_count_phrase(unchanged_count, "unchanged", "unchanged"),
    ];
    let visible = if verbose {
        phrases.into_iter().collect::<Vec<_>>()
    } else {
        phrases
            .into_iter()
            .filter(|phrase| !phrase.starts_with("0 "))
            .collect::<Vec<_>>()
    };
    visible.join(" • ")
}

fn short_revision(revision: &str) -> &str {
    &revision[..revision.len().min(8)]
}

fn display_scope_name(scope_id: &str) -> &str {
    scope_id.strip_prefix("host:").unwrap_or(scope_id)
}

fn color_reset() -> &'static str { "\x1b[0m" }
fn color_header() -> &'static str { "\x1b[1m\x1b[37m" }
fn color_create() -> &'static str { "\x1b[32m" }
fn color_update() -> &'static str { "\x1b[33m" }
fn color_delete() -> &'static str { "\x1b[31m" }
fn color_restart() -> &'static str { "\x1b[34m" }
fn color_blocked() -> &'static str { "\x1b[35m" }
fn color_noop() -> &'static str { "\x1b[2m\x1b[37m" }

const DEFAULT_DIFF_CONTEXT_LINES: usize = 6;

fn render_group(
    output: &mut String,
    plan: &DeterministicReconciliationPlan,
    entries: &[PlanEntry],
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
    label_column: usize,
    action: PlanEntryAction,
    title: &str,
    verbose: bool,
) {
    let group = entries
        .iter()
        .filter(|entry| entry.action == action)
        .collect::<Vec<_>>();
    if group.is_empty() {
        return;
    }
    output.push_str(&format!(
        "{}{} {} • {}{}\n\n",
        color_header(),
        colored_marker(&action),
        title,
        group.len(),
        color_reset(),
    ));
    for entry in group {
        let has_details = render_entry(
            output,
            plan,
            entry,
            entry_by_name,
            label_column,
            verbose,
        );
        if has_details {
            output.push('\n');
        }
    }
    output.push('\n');
}

fn render_entry(
    output: &mut String,
    plan: &DeterministicReconciliationPlan,
    entry: &PlanEntry,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
    label_column: usize,
    verbose: bool,
) -> bool {
    output.push_str(&format_plan_label_line(
        "",
        Some(colored_marker(&entry.action)),
        &entry.object.display_id,
        short_label(entry),
        label_column,
    ));
    let mut has_details = false;
    if let Some(cause) = full_cause_line(entry) {
        output.push_str(&format!("    {}\n", cause));
        has_details = true;
    }
    if !entry.dependencies.is_empty() {
        output.push_str("    requires\n");
        output.push_str(&render_dependency_tree(
            plan,
            &entry.object.name,
            "      ",
            entry_by_name,
            label_column,
        ));
        has_details = true;
    }
    if let Some(diff) = &entry.diff {
        output.push_str(&format!(
            "    Δ {}\n",
            displayed_diff_summary(diff, verbose)
        ));
        render_diff_body(output, diff, verbose);
        has_details = true;
    }
    has_details
}

fn displayed_diff_summary(diff: &SemanticDiffView, verbose: bool) -> String {
    if verbose {
        return diff.summary.clone();
    }
    if let Some(unified_diff) = &diff.unified_diff {
        let stats = diff_hunk_stats(unified_diff);
        if stats.total() > DEFAULT_DIFF_CONTEXT_LINES {
            if stats.deletions == 0 && stats.additions > 0 {
                return format!("{} ({} additions)", diff.summary, stats.additions);
            }
            if stats.additions == 0 && stats.deletions > 0 {
                return format!("{} ({} deletions)", diff.summary, stats.deletions);
            }
            return format!(
                "{} ({} additions, {} deletions)",
                diff.summary, stats.additions, stats.deletions
            );
        }
    }
    diff.summary.clone()
}

fn render_diff_body(output: &mut String, diff: &SemanticDiffView, verbose: bool) {
    if let Some(unified_diff) = &diff.unified_diff {
        let lines = unified_diff.lines().collect::<Vec<_>>();
        if verbose || diff_hunk_stats(unified_diff).total() <= DEFAULT_DIFF_CONTEXT_LINES {
            for line in lines {
                output.push_str("      ");
                output.push_str(line);
                output.push('\n');
            }
        } else {
            for line in lines.into_iter().skip(2).take(DEFAULT_DIFF_CONTEXT_LINES) {
                output.push_str("      ");
                output.push_str(line);
                output.push('\n');
            }
            output.push_str("      ...\n");
        }
    } else if let Some(details) = &diff.details {
        for field in significant_diff_fields_for_details(details) {
            let details = details
                .get(&field)
                .expect("significant field must exist in diff details");
            output.push_str(&format!("      {}: {}\n", field, compact_diff_details(details)));
        }
    }
}

struct DiffHunkStats {
    additions: usize,
    deletions: usize,
}

impl DiffHunkStats {
    fn total(&self) -> usize {
        self.additions + self.deletions
    }
}

fn diff_hunk_stats(unified_diff: &str) -> DiffHunkStats {
    let mut additions = 0usize;
    let mut deletions = 0usize;
    for line in unified_diff.lines().skip(2) {
        if line.starts_with("+ ") {
            additions += 1;
        } else if line.starts_with("- ") {
            deletions += 1;
        }
    }
    DiffHunkStats { additions, deletions }
}

fn plan_label_column(
    plan: &DeterministicReconciliationPlan,
    entries: &[PlanEntry],
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
) -> usize {
    let mut column = 0usize;
    for entry in entries {
        column = column.max(plan_label_prefix_width(
            "",
            Some(colored_marker(&entry.action)),
            &entry.object.display_id,
        ));
        column = column.max(dependency_tree_label_column(
            plan,
            &entry.object.name,
            "      ",
            entry_by_name,
        ));
    }
    column + 8
}

fn dependency_tree_label_column(
    plan: &DeterministicReconciliationPlan,
    object_id: &str,
    indent: &str,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
) -> usize {
    let mut column = 0usize;
    let mut children = direct_prerequisite_refs(&plan.graph, object_id)
        .into_iter()
        .map(|object| (DependencyRelation::Prerequisite, object))
        .collect::<Vec<_>>();
    if let Some(action) = plan.actions.iter().find(|action| action.object_id == object_id) {
        let direct_ids = children
            .iter()
            .map(|(_, object)| object.name.clone())
            .collect::<BTreeSet<_>>();
        children.extend(action.dependency_context.iter().filter_map(|dependency| {
            (!direct_ids.contains(dependency)).then(|| {
                (DependencyRelation::Blocker, inferred_managed_object_ref(dependency))
            })
        }));
    }
    children.sort_by(|a, b| a.1.display_id.cmp(&b.1.display_id));
    for (index, (_, object)) in children.iter().enumerate() {
        let branch = if index + 1 == children.len() { "└─" } else { "├─" };
        let entry = entry_by_name.get(object.name.as_str()).copied();
        column = column.max(plan_label_prefix_width(
            &format!("{}{} ", indent, branch),
            entry.map(|entry| colored_marker(&entry.action)),
            &object.display_id,
        ));
        column = column.max(dependency_tree_label_column(
            plan,
            &object.name,
            &format!("{indent}   "),
            entry_by_name,
        ));
    }
    column
}

fn format_plan_label_line(
    prefix: &str,
    marker: Option<String>,
    display_id: &str,
    label: &str,
    label_column: usize,
) -> String {
    let marker = marker.expect("plan label line requires a marker");
    let visible_prefix_width = plan_label_prefix_width(prefix, Some(marker.clone()), display_id);
    let tabs = tabs_to_reach(visible_prefix_width, label_column);
    format!("{prefix}{marker} {display_id}{tabs}{label}\n")
}

fn plan_label_prefix_width(prefix: &str, marker: Option<String>, display_id: &str) -> usize {
    let marker_width = marker.map(|marker| visible_width(&marker) + 1).unwrap_or(0);
    prefix.chars().count() + marker_width + display_id.chars().count()
}

fn tabs_to_reach(current_width: usize, target_column: usize) -> String {
    let mut width = current_width;
    let mut tabs = String::new();
    while width < target_column {
        tabs.push('\t');
        width = ((width / 8) + 1) * 8;
    }
    tabs
}

fn visible_width(value: &str) -> usize {
    let mut width = 0usize;
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}


pub fn format_deterministic_plan_json(plan: &DeterministicReconciliationPlan) -> String {
    serde_json::to_string(&build_plan_output(plan)).expect("serialize plan output")
}

pub fn format_rollback_report(
    target: &RollbackTargetCandidate,
    plan: &DeterministicReconciliationPlan,
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "rollback target={} eligibility={:?}\n",
        target.target_revision_id, target.eligibility
    ));
    output.push_str(&format_deterministic_plan_report(plan));
    output
}

fn quadlet_type_label(quadlet_type: Option<QuadletType>) -> &'static str {
    match quadlet_type {
        Some(QuadletType::Container) => "container",
        Some(QuadletType::Socket) => "socket",
        Some(QuadletType::SocketDropIn) => "socket-dropin",
        Some(QuadletType::ConfigFile) => "config",
        Some(QuadletType::Mount) => "mount",
        Some(QuadletType::Automount) => "automount",
        Some(QuadletType::Volume) => "volume",
        Some(QuadletType::Pod) => "pod",
        Some(QuadletType::Network) => "network",
        None => "unknown",
    }
}

fn action_label(
    action: &crate::core::types::PlanActionType,
    quadlet_type: Option<&QuadletType>,
) -> String {
    match action {
        crate::core::types::PlanActionType::PreparePath => "PreparePath".to_string(),
        crate::core::types::PlanActionType::WriteQuadlet => {
            if matches!(
                quadlet_type,
                Some(QuadletType::Socket)
                    | Some(QuadletType::SocketDropIn)
                    | Some(QuadletType::Mount)
                    | Some(QuadletType::Automount)
            ) {
                "WriteUnit".to_string()
            } else {
                "WriteQuadlet".to_string()
            }
        }
        crate::core::types::PlanActionType::RemoveQuadlet => {
            if matches!(
                quadlet_type,
                Some(QuadletType::Socket)
                    | Some(QuadletType::SocketDropIn)
                    | Some(QuadletType::Mount)
                    | Some(QuadletType::Automount)
            ) {
                "RemoveUnit".to_string()
            } else {
                "RemoveQuadlet".to_string()
            }
        }
        _ => format!("{:?}", action),
    }
}

fn dependency_relation_label(relation: &DependencyRelation) -> &'static str {
    match relation {
        DependencyRelation::Prerequisite => "prerequisite",
        DependencyRelation::Dependent => "dependent",
        DependencyRelation::Blocker => "blocker",
    }
}

fn run_status_label(status: &crate::core::types::RunStatus) -> &'static str {
    match status {
        crate::core::types::RunStatus::Success => "success",
        crate::core::types::RunStatus::Failure => "failure",
    }
}

fn verification_status_label(status: &crate::core::types::VerificationStatus) -> &'static str {
    match status {
        crate::core::types::VerificationStatus::Success => "success",
        crate::core::types::VerificationStatus::Failure => "failure",
    }
}

fn convergence_status_label(status: &crate::core::types::ConvergenceStatus) -> &'static str {
    match status {
        crate::core::types::ConvergenceStatus::Success => "success",
        crate::core::types::ConvergenceStatus::Partial => "partial",
        crate::core::types::ConvergenceStatus::Blocked => "blocked",
        crate::core::types::ConvergenceStatus::RepeatedFailure => "repeated_failure",
        crate::core::types::ConvergenceStatus::Oscillation => "oscillation",
        crate::core::types::ConvergenceStatus::Failed => "failed",
    }
}
