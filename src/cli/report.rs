use crate::core::planner::{
    dependency_edges_for_object, dependent_refs, direct_prerequisite_refs, managed_object_ref,
    object_kind_by_id,
};
use crate::core::release_governance::{GovernanceEvaluationResult, GovernanceStatus, ReleaseClassification, ReleaseIntent};
use crate::core::types::{
    ApplyOutputView, ApplyPhaseEvent, ApplyPhaseKind, Cause, CauseKind, DependencyEdgeView,
    DependencyRelation, DeterministicActionClass, DeterministicConvergenceRecord,
    DeterministicReconciliationPlan, DiffItem, ExecutionEvent, ExecutionEventKind, ExecutionState,
    ExplainDependencyView, ExplainOutputView, ManagedObjectKind, ManagedObjectRef, PhaseState,
    PlanEntry, PlanEntryAction, PlanOutputView, PlanSummaryView, QuadletType, ReconcileRun,
    ReconciliationPlan, ResultEntry, ResultFinalState, ResultOutcome, ResultOutputView,
    ResultSummaryView, RevisionContext, RollbackTargetCandidate, SemanticDiffKind,
    SemanticDiffView, VerificationResult, VerificationRunMode, VerificationRunOutcome,
};
use crate::core::unit::systemd_unit_for_quadlet_file;
use crate::core::validation::{
    validate_apply_output_view, validate_explain_output_view, validate_plan_output_view,
    validate_result_output_view,
};
use crate::core::verification_generate::VerificationCoverageReport;
use crate::core::verification_model::{VerificationRunView, VerificationScenarioClass};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyHumanMode {
    Default,
    Verbose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyRunDisplayState {
    Managed,
    FirstRun,
    Recovery,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyInteractiveEvent {
    Begin(String),
    Started { target: String, line: String },
    Terminal { target: String, block: String },
    Finish(String),
}

impl fmt::Display for ApplyInteractiveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplyInteractiveEvent::Begin(text) | ApplyInteractiveEvent::Finish(text) => {
                f.write_str(text)
            }
            ApplyInteractiveEvent::Started { line, .. } => f.write_str(line),
            ApplyInteractiveEvent::Terminal { block, .. } => f.write_str(block),
        }
    }
}

pub struct ApplyProgressRenderer {
    plan: DeterministicReconciliationPlan,
    plan_view: PlanOutputView,
    entry_by_name: BTreeMap<String, PlanEntry>,
    label_column: usize,
    mode: ApplyHumanMode,
    run_display_state: ApplyRunDisplayState,
    streamed_started: BTreeSet<String>,
    streamed_terminal: BTreeSet<String>,
}

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
        Some(contents) => format!(
            "{base}\n{}",
            crate::cli::status::format_status_text(contents)
        ),
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

pub fn format_verification_run_report(view: &VerificationRunView) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Verification run {} [{}]\n",
        view.scenario_id,
        verification_mode_label(view.mode)
    ));
    output.push_str(&format!("{}\n\n", "─".repeat(18 + view.scenario_id.len())));
    output.push_str(&format!("Title:   {}\n", view.title));
    output.push_str(&format!(
        "Outcome: {}\n",
        verification_outcome_label(view.overall_outcome)
    ));
    output.push_str(&format!("Controller: {}\n", view.controller_version));
    output.push_str(&format!("Run ID:  {}\n", view.run_id));
    output.push_str(&format!("Bundle:  {}\n", view.artifact_bundle.bundle_path));
    output.push_str(&format!(
        "Env:     {}\n",
        if view.environment_retained {
            "retained"
        } else {
            "torn down"
        }
    ));
    output.push_str(&format!(
        "Artifacts: {}\n",
        match view.artifact_bundle.collection_status {
            crate::core::types::VerificationArtifactCollectionStatus::Complete => "complete",
            crate::core::types::VerificationArtifactCollectionStatus::Partial => "partial",
            crate::core::types::VerificationArtifactCollectionStatus::Failed => "failed",
        }
    ));
    if let Some(summary) = &view.failure_summary {
        output.push_str(&format!("Failure: {summary}\n"));
    }
    if let Some(readiness) = &view.readiness_evidence {
        output.push_str(&format!(
            "Readiness: {} via {}\n",
            readiness.final_status, readiness.source
        ));
        if let Some(record) = &readiness.accepted_record {
            output.push_str(&format!("Readiness IPv4: {}\n", record.ip));
        }
        if !readiness.rejected_records.is_empty() {
            output.push_str(&format!(
                "Readiness Rejections: {}\n",
                readiness.rejected_records.len()
            ));
        }
    }
    if let Some(summary) = &view.regression_summary {
        let compact = summary
            .lines()
            .find(|line| line.starts_with("Revision sequence: "))
            .map(|line| line.trim_start_matches("Revision sequence: "))
            .unwrap_or(summary.as_str());
        output.push_str(&format!("Regression: {compact}\n"));
    }
    if let Some(status) = &view.promotion_status {
        output.push_str(&format!("Promotion: {status}\n"));
    }
    if !view.artifact_bundle.failure_specific_entries.is_empty() {
        output.push_str("Failure-Specific Artifacts:\n");
        for entry in &view.artifact_bundle.failure_specific_entries {
            output.push_str(&format!("  - {entry}\n"));
        }
    }
    if let Some(step) = view
        .step_results
        .iter()
        .find(|step| step.status == crate::core::types::VerificationStepStatus::Failed)
    {
        output.push_str(&format!("Failed Step: {}\n", step.step_id));
        if let Some(command) = &step.command {
            output.push_str(&format!("Command: {}\n", command));
        }
        if let Some(exit_code) = step.exit_code {
            output.push_str(&format!("Exit Code: {}\n", exit_code));
        }
        if let Some(duration_ms) = step.duration_ms {
            output.push_str(&format!("Duration: {} ms\n", duration_ms));
        }
        if let Some(stderr) = &step.stderr {
            if !stderr.trim().is_empty() {
                output.push_str("Stderr:\n");
                output.push_str(stderr.trim());
                output.push('\n');
            }
        }
        if let Some(stdout) = &step.stdout {
            if !stdout.trim().is_empty() {
                output.push_str("Stdout:\n");
                output.push_str(stdout.trim());
                output.push('\n');
            }
        }
    }
    if !view.warnings.is_empty() {
        output.push_str("\nWarnings\n────────\n");
        for warning in &view.warnings {
            output.push_str(&format!("- {warning}\n"));
        }
    }
    output
}

pub fn format_verification_run_json(view: &VerificationRunView) -> String {
    serde_json::json!({
        "view_kind": &view.view_kind,
        "run_id": &view.run_id,
        "mode": view.mode,
        "controller_version": &view.controller_version,
        "revision_selection_basis": view.revision_selection_basis,
        "revision_under_test": &view.revision_under_test,
        "overall_outcome": view.overall_outcome,
        "started_at": &view.started_at,
        "completed_at": &view.completed_at,
        "scenario_outcomes": [
            {
                "scenario_id": &view.scenario_id,
                "revision_under_test": &view.revision_under_test,
                "outcome": view.overall_outcome,
                "failure_summary": &view.failure_summary,
                "readiness_evidence": &view.readiness_evidence,
                "assertion_results": view.assertion_results.iter().map(|result| serde_json::json!({
                    "assertion_id": &result.assertion_id,
                    "status": result.status,
                    "observed_value": &result.observed_value,
                    "evidence_refs": &result.evidence_refs,
                })).collect::<Vec<_>>(),
            }
        ],
        "artifacts": {
            "bundle_path": &view.artifact_bundle.bundle_path,
            "environment_retained": view.environment_retained,
        }
    })
    .to_string()
}

pub fn format_release_governance_report(result: &GovernanceEvaluationResult) -> String {
    let mut output = String::new();
    output.push_str("Release Governance\n");
    output.push_str("──────────────────\n");
    output.push_str(&format!(
        "Outcome: {}\n",
        match result.overall_status {
            GovernanceStatus::Passed => "passed",
            GovernanceStatus::Failed => "failed",
        }
    ));
    output.push_str(&format!(
        "Classification: {}\n",
        match result.effective_classification {
            ReleaseClassification::Exempt => "exempt",
            ReleaseClassification::Releasable => "releasable",
        }
    ));
    output.push_str(&format!(
        "Required bump: {}\n",
        result
            .effective_bump
            .map(|intent| intent.label())
            .unwrap_or("none")
    ));
    output.push_str(&format!(
        "Declared bump: {}\n",
        result
            .declared_bump
            .map(|intent| intent.label())
            .unwrap_or("none")
    ));
    output.push_str(&format!(
        "Version bump: {}\n",
        result
            .version_bump
            .map(|intent| intent.label())
            .unwrap_or("none")
    ));
    output.push_str(&format!(
        "Metadata-only: {}\n",
        if result.metadata_only { "yes" } else { "no" }
    ));
    output.push_str(&format!(
        "Release preparation: {}\n",
        if result.release_preparation {
            "true"
        } else {
            "false"
        }
    ));
    output.push_str(&format!(
        "CHANGELOG aligned: {}\n",
        if result.changelog_aligned { "yes" } else { "no" }
    ));

    if !result.changed_paths.is_empty() {
        output.push_str("Changed Paths:\n");
        for path in &result.changed_paths {
            output.push_str(&format!("  - {path}\n"));
        }
    }
    if !result.changed_fragment_paths.is_empty() {
        output.push_str("Changed Fragments:\n");
        for path in &result.changed_fragment_paths {
            output.push_str(&format!("  - {path}\n"));
        }
    }
    if !result.applied_rules.is_empty() {
        output.push_str("Applied Rules:\n");
        for rule in &result.applied_rules {
            output.push_str(&format!("  - {rule}\n"));
        }
    }
    if !result.missing_artifacts.is_empty() {
        output.push_str("Missing Artifacts:\n");
        for artifact in &result.missing_artifacts {
            output.push_str(&format!("  - {artifact}\n"));
        }
    }
    if !result.mismatch_reasons.is_empty() {
        output.push_str("Mismatches:\n");
        for reason in &result.mismatch_reasons {
            output.push_str(&format!("  - {reason}\n"));
        }
    }
    output
}

pub fn format_release_governance_json(result: &GovernanceEvaluationResult) -> String {
    serde_json::json!({
        "view_kind": "release_governance",
        "overall_status": match result.overall_status {
            GovernanceStatus::Passed => "passed",
            GovernanceStatus::Failed => "failed",
        },
        "effective_classification": match result.effective_classification {
            ReleaseClassification::Exempt => "exempt",
            ReleaseClassification::Releasable => "releasable",
        },
        "effective_bump": result.effective_bump.map(ReleaseIntent::label),
        "declared_bump": result.declared_bump.map(ReleaseIntent::label),
        "version_bump": result.version_bump.map(ReleaseIntent::label),
        "missing_artifacts": &result.missing_artifacts,
        "mismatch_reasons": &result.mismatch_reasons,
        "applied_rules": &result.applied_rules,
        "changed_paths": &result.changed_paths,
        "changed_fragment_paths": &result.changed_fragment_paths,
        "release_preparation": result.release_preparation,
        "metadata_only": result.metadata_only,
        "changelog_aligned": result.changelog_aligned,
    })
    .to_string()
}

pub fn format_release_governance_changelog_report(
    output_path: &std::path::Path,
    aligned: bool,
    wrote: bool,
) -> String {
    if wrote {
        format!(
            "Generated changelog written to {}\n",
            output_path.display()
        )
    } else if aligned {
        format!("CHANGELOG aligned: {}\n", output_path.display())
    } else {
        format!("CHANGELOG not aligned: {}\n", output_path.display())
    }
}

pub fn format_verification_suite_report(
    run_id: &str,
    mode: VerificationRunMode,
    overall_outcome: VerificationRunOutcome,
    revision_under_test: &str,
    scenario_outcomes: &[crate::core::verification_model::VerificationScenarioOutcome],
    bundle_path: &str,
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Verification suite [{}]\n",
        verification_mode_label(mode)
    ));
    output.push_str("────────────────────\n\n");
    output.push_str(&format!("Run ID:  {run_id}\n"));
    output.push_str(&format!(
        "Outcome: {}\n",
        verification_outcome_label(overall_outcome)
    ));
    output.push_str(&format!("Revision: {revision_under_test}\n"));
    output.push_str(&format!("Bundle:  {bundle_path}\n"));
    output.push_str(&format!("Scenarios: {}\n", scenario_outcomes.len()));
    output.push_str("\nScenario Outcomes\n─────────────────\n");
    for outcome in scenario_outcomes {
        output.push_str(&format!(
            "- {} @ {}: {}\n",
            outcome.scenario_id,
            outcome.revision_under_test,
            verification_outcome_label(outcome.outcome)
        ));
    }
    output
}

pub fn format_verification_coverage_report(report: &VerificationCoverageReport) -> String {
    let mut output = String::new();
    output.push_str("\nCoverage\n────────\n");
    output.push_str(&format!(
        "Required: {}\n",
        format_scenario_classes(&report.required_classes)
    ));
    output.push_str(&format!(
        "Covered:  {}\n",
        format_scenario_classes(&report.covered_classes)
    ));
    output.push_str(&format!(
        "Missing:  {}\n",
        format_scenario_classes(&report.missing_classes)
    ));
    output
}

pub fn build_apply_output(
    plan: &DeterministicReconciliationPlan,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
) -> ApplyOutputView {
    let plan_view = build_plan_output(plan);
    let phases = build_apply_phase_events(convergence);
    let mut next_sequence = phases.len();
    let mut events = Vec::new();

    for entry in &plan_view.entries {
        match entry.action {
            PlanEntryAction::NoOp => {
                let failed = convergence_failed_for_entry(entry, convergence)
                    || verification_failed_for_entry(entry, verification_results);
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectTerminal,
                    state: if failed {
                        ExecutionState::Failed
                    } else {
                        ExecutionState::Unchanged
                    },
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: failed
                        .then(|| failure_cause_for_entry(entry, verification_results))
                        .flatten(),
                    phase: Some(if failed {
                        ApplyPhaseKind::ConvergenceCheck
                    } else {
                        ApplyPhaseKind::Execution
                    }),
                    impacted_objects: None,
                });
                next_sequence += 1;
            }
            PlanEntryAction::Skipped => {
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectSkipped,
                    state: ExecutionState::Skipped,
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: None,
                    phase: Some(ApplyPhaseKind::Execution),
                    impacted_objects: None,
                });
                next_sequence += 1;
            }
            PlanEntryAction::Blocked => {
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectTerminal,
                    state: ExecutionState::Blocked,
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: failure_cause_for_entry(entry, verification_results)
                        .or_else(|| entry.causes.first().cloned()),
                    phase: Some(ApplyPhaseKind::Execution),
                    impacted_objects: impacted_objects_for_entry(entry),
                });
                next_sequence += 1;
            }
            _ => {
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectProgress,
                    state: ExecutionState::Pending,
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: None,
                    phase: Some(ApplyPhaseKind::Execution),
                    impacted_objects: None,
                });
                next_sequence += 1;
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectProgress,
                    state: ExecutionState::Running,
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: None,
                    phase: Some(ApplyPhaseKind::Execution),
                    impacted_objects: None,
                });
                next_sequence += 1;
                // FR-005: RestartUnit is always present in the executable plan when
                // the deterministic plan shows Restart for config-file-dependent containers
                // (see planner.rs dependent-restart pass); restart failures surface via
                // failed_actions → convergence_failed_for_entry.
                let failed = convergence_failed_for_entry(entry, convergence)
                    || verification_failed_for_entry(entry, verification_results);
                events.push(ExecutionEvent {
                    object: entry.object.clone(),
                    event_kind: ExecutionEventKind::ObjectTerminal,
                    state: if failed {
                        ExecutionState::Failed
                    } else {
                        terminal_execution_state(&entry.action)
                    },
                    sequence: next_sequence,
                    action: Some(entry.action.clone()),
                    cause: if failed {
                        failure_cause_for_entry(entry, verification_results)
                            .or_else(|| entry.causes.first().cloned())
                    } else {
                        entry.causes.first().cloned()
                    },
                    phase: Some(if failed {
                        ApplyPhaseKind::ConvergenceCheck
                    } else {
                        ApplyPhaseKind::Execution
                    }),
                    impacted_objects: failed
                        .then(|| {
                            entry
                                .dependencies
                                .iter()
                                .map(|edge| edge.object.clone())
                                .collect::<Vec<_>>()
                        })
                        .filter(|objects| !objects.is_empty()),
                });
                next_sequence += 1;
            }
        }
    }

    let view = ApplyOutputView {
        view_kind: "apply".to_string(),
        revision_context: plan_view.revision_context,
        phases,
        events,
        summary: Some(plan_view.summary),
    };
    validate_apply_output_view(&view).expect("apply output view must remain valid");
    view
}

pub fn format_apply_output_json(
    plan: &DeterministicReconciliationPlan,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
) -> String {
    serde_json::to_string(&build_apply_output(plan, verification_results, convergence))
        .expect("serialize apply output")
}

pub fn build_result_output(
    plan: &DeterministicReconciliationPlan,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
) -> ResultOutputView {
    let plan_view = build_plan_output(plan);
    let apply_view = build_apply_output(plan, verification_results, convergence);
    let terminal_by_name = terminal_apply_events(&apply_view)
        .into_iter()
        .map(|event| (event.object.name.clone(), event))
        .collect::<BTreeMap<_, _>>();

    let entries = plan_view
        .entries
        .iter()
        .map(|entry| {
            let terminal = terminal_by_name
                .get(entry.object.name.as_str())
                .expect("terminal event for every planned object");
            ResultEntry {
                object: entry.object.clone(),
                final_state: result_final_state_for_event(terminal),
                action: Some(entry.action.clone()),
                causes: (!entry.causes.is_empty()).then(|| entry.causes.clone()),
                dependencies: (!entry.dependencies.is_empty()).then(|| entry.dependencies.clone()),
                diff: entry.diff.clone(),
            }
        })
        .collect::<Vec<_>>();

    let summary = ResultSummaryView {
        changed_count: entries
            .iter()
            .filter(|entry| !matches!(entry.final_state, ResultFinalState::NoOp))
            .count(),
        failed_count: entries
            .iter()
            .filter(|entry| matches!(entry.final_state, ResultFinalState::Failed))
            .count(),
        blocked_count: entries
            .iter()
            .filter(|entry| matches!(entry.final_state, ResultFinalState::Blocked))
            .count(),
        skipped_count: entries
            .iter()
            .filter(|entry| matches!(entry.final_state, ResultFinalState::Skipped))
            .count(),
        unchanged_count: entries
            .iter()
            .filter(|entry| matches!(entry.final_state, ResultFinalState::NoOp))
            .count(),
        message: convergence.map(|record| convergence_outcome_label(&record.status).to_string()),
    };

    let view = ResultOutputView {
        view_kind: "result".to_string(),
        revision_context: plan_view.revision_context,
        outcome: result_outcome_for_convergence(convergence),
        summary,
        entries,
    };
    validate_result_output_view(&view).expect("result output view must remain valid");
    view
}

pub fn format_result_output_json(view: &ResultOutputView) -> String {
    serde_json::to_string(view).expect("serialize result output")
}

pub fn format_result_output_report(view: &ResultOutputView) -> String {
    let label_column = view
        .entries
        .iter()
        .map(|entry| visible_width(&entry.object.display_id))
        .max()
        .unwrap_or(0)
        + 16;
    let header = format!(
        "Result for host {} @ {}",
        display_scope_name_from_revision_context(&view.revision_context),
        result_header_revision_context(&view.revision_context),
    );
    let mut output = String::new();
    output.push_str(&format!("{}{}{}\n", color_header(), header, color_reset()));
    output.push_str(&format!("{}\n", "─".repeat(header.chars().count())));
    output.push('\n');
    for entry in &view.entries {
        output.push_str(&format_plan_label_line(
            "",
            Some(result_marker(entry)),
            &entry.object.display_id,
            result_label(entry),
            label_column,
        ));
        if let Some(causes) = &entry.causes {
            if let Some(cause) = causes.first() {
                output.push_str(&format!("    because {}\n", cause.summary));
            }
        }
    }
    output.push('\n');
    let summary_heading = "Summary";
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        summary_heading,
        color_reset(),
    ));
    output.push_str(&format!(
        "{}\n",
        "─".repeat(summary_heading.chars().count())
    ));
    output.push_str(&format!(
        "{}\nOutcome: {}\n",
        result_summary_line(&view.summary),
        result_outcome_label(&view.outcome),
    ));
    output
}

pub fn build_explain_output(
    plan: &DeterministicReconciliationPlan,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
    object_selector: &str,
) -> Option<ExplainOutputView> {
    let result = build_result_output(plan, verification_results, convergence);
    let result_entries = result
        .entries
        .iter()
        .map(|entry| (entry.object.name.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let entry = result.entries.iter().find(|entry| {
        entry.object.display_id == object_selector
            || entry.object.name == object_selector
            || entry.object.display_id.ends_with(object_selector)
    })?;
    let dependencies = inspect_plan_dependencies(plan, &entry.object.name);
    let explain = ExplainOutputView {
        view_kind: "explain".to_string(),
        revision_context: result.revision_context.clone(),
        object: entry.object.clone(),
        action_or_outcome: explain_action_or_outcome(entry),
        causes: entry.causes.clone().unwrap_or_default(),
        dependencies: dependencies.clone(),
        dependency_context: Some(
            dependencies
                .iter()
                .map(|dependency| ExplainDependencyView {
                    relation: dependency.relation.clone(),
                    object: dependency.object.clone(),
                    state: result_entries
                        .get(dependency.object.name.as_str())
                        .map(|entry| explain_action_or_outcome(entry))
                        .unwrap_or_else(|| "unknown".to_string()),
                    reason: result_entries
                        .get(dependency.object.name.as_str())
                        .and_then(|entry| entry.causes.as_ref())
                        .and_then(|causes| causes.first())
                        .map(humane_cause_summary)
                        .unwrap_or_else(|| {
                            result_entries
                                .get(dependency.object.name.as_str())
                                .map(|entry| default_explain_reason(entry))
                                .unwrap_or_else(|| "dependency context available".to_string())
                        }),
                })
                .collect(),
        ),
        diff: entry.diff.clone(),
        metadata: Some(explain_metadata(entry)),
        x_coreops: explain_x_coreops(entry),
        apply_intent: Some(explain_apply_intent(entry, convergence.is_some())),
        summary: Some(explain_summary(entry, convergence.is_some())),
        history: None,
    };
    validate_explain_output_view(&explain).expect("explain output view must remain valid");
    Some(explain)
}

pub fn format_explain_output_json(view: &ExplainOutputView) -> String {
    serde_json::to_string(view).expect("serialize explain output")
}

pub fn format_explain_output_report(view: &ExplainOutputView) -> String {
    let header = format!("Explain: {}", view.object.display_id);
    let mut output = String::new();
    output.push_str(&format!("{}{}{}\n", color_header(), header, color_reset()));
    output.push_str(&format!("{}\n", "─".repeat(header.chars().count())));
    output.push_str("\nState\n─────\n");
    output.push_str(&format!(
        "Action: {}\n",
        explain_humane_state_label(&view.action_or_outcome)
    ));
    output.push_str(&format!(
        "Reason: {}\n",
        view.causes
            .first()
            .map(humane_cause_summary)
            .unwrap_or_else(|| default_explain_reason_from_view(view))
    ));
    if let Some(runtime) = explain_runtime_line(view) {
        output.push_str(&format!("Runtime: {}\n", runtime));
    }
    if let Some(apply_intent) = &view.apply_intent {
        output.push_str(&format!("Apply intent: {}\n", apply_intent));
    }
    if let Some(context) = format_explain_context(view) {
        output.push_str("\nContext\n───────\n");
        output.push_str(&context);
    }
    output.push_str("\nIdentity\n────────\n");
    output.push_str(&format!("Object: {}\n", view.object.display_id));
    output.push_str(&format!("Type:   {}\n", view.object.resource_type));
    if let Some(metadata) = &view.metadata {
        if let Some(unit) = metadata.get("runtime_unit") {
            output.push_str(&format!("Unit:   {}\n", unit));
        }
    }
    if let Some(dependencies) = &view.dependency_context {
        if !dependencies.is_empty() {
            output.push_str("\nDependency context\n──────────────────\n");
            for relation in [
                DependencyRelation::Prerequisite,
                DependencyRelation::Dependent,
                DependencyRelation::Blocker,
            ] {
                let related = dependencies
                    .iter()
                    .filter(|dependency| dependency.relation == relation)
                    .collect::<Vec<_>>();
                if related.is_empty() {
                    continue;
                }
                output.push_str(&format!("{}\n", dependency_relation_heading(&relation)));
                for (index, dependency) in related.iter().enumerate() {
                    let is_last = index + 1 == related.len();
                    let branch = if is_last { "  └─" } else { "  ├─" };
                    let continuation = if is_last { "     " } else { "  │  " };
                    output.push_str(&format!("{branch} {}\n", dependency.object.display_id));
                    output.push_str(&format!("{continuation}state: {}\n", dependency.state));
                    output.push_str(&format!("{continuation}reason: {}\n", dependency.reason));
                }
            }
        }
    }
    if let Some(diff) = &view.diff {
        output.push_str("\nChange Context\n──────────────\n");
        output.push_str(&format!("{}\n", diff.summary));
    }
    if let Some(x_coreops) = &view.x_coreops {
        output.push_str("\nX-CoreOps\n─────────\n");
        for (key, value) in x_coreops {
            output.push_str(&format!("{key}: {}\n", json_value_to_humane(value)));
        }
    }
    if let Some(summary) = &view.summary {
        output.push_str("\nSummary\n───────\n");
        output.push_str(&format!("{summary}\n"));
    }
    output
}

pub fn render_apply_output_from_events(view: &ApplyOutputView, mode: ApplyHumanMode) -> String {
    let label_column = terminal_apply_events(view)
        .iter()
        .map(|event| visible_width(&event.object.display_id))
        .max()
        .unwrap_or(0)
        + 16;
    let header = format!(
        "Apply for host {} @ {}",
        display_scope_name_from_revision_context(&view.revision_context),
        result_header_revision_context(&view.revision_context),
    );
    let mut output = String::new();
    output.push_str(&format!("{}{}{}\n", color_header(), header, color_reset()));
    output.push_str(&format!("{}\n\n", "─".repeat(header.chars().count())));
    let execution_heading = "Execution";
    output.push_str(&format!(
        "{}{}{}\n{}\n",
        color_header(),
        execution_heading,
        color_reset(),
        "─".repeat(execution_heading.chars().count())
    ));
    for event in terminal_apply_events(view) {
        if !apply_event_visible(&event, mode) {
            continue;
        }
        output.push_str(&format_plan_label_line(
            "",
            Some(colored_marker_from_execution_state(&event.state)),
            &event.object.display_id,
            execution_state_label(&event.state),
            label_column,
        ));
        if let Some(cause) = &event.cause {
            if !matches!(event.state, ExecutionState::Unchanged) {
                output.push_str(&format!("    because {}\n", cause.summary));
            }
        }
    }
    output.push('\n');
    let summary_heading = "Summary";
    output.push_str(&format!(
        "{}{}{}\n{}\n",
        color_header(),
        summary_heading,
        color_reset(),
        "─".repeat(summary_heading.chars().count())
    ));
    output.push_str(&format!(
        "{}\nOutcome: {}\n",
        crate::cli::status::render_apply_count_summary(view),
        apply_outcome_label_from_view(view)
    ));
    output
}

pub fn format_apply_output_report(
    plan: &DeterministicReconciliationPlan,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
    mode: ApplyHumanMode,
    run_display_state: ApplyRunDisplayState,
) -> String {
    let view = build_apply_output(plan, verification_results, convergence);
    let plan_view = build_plan_output(plan);
    let entry_by_name = plan_view
        .entries
        .iter()
        .map(|entry| (entry.object.name.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let terminal_events = terminal_apply_events(&view);
    let label_column = terminal_events
        .iter()
        .map(|event| visible_width(&event.object.display_id))
        .max()
        .unwrap_or(0)
        + 16;
    let header = format!(
        "Apply for host {} @ {}",
        display_scope_name(&plan.scope_id),
        apply_header_revision_context(&view.revision_context, run_display_state),
    );
    let mut output = String::new();
    output.push_str(&format!("{}{}{}\n", color_header(), header, color_reset(),));
    output.push_str(&format!("{}\n", "─".repeat(header.chars().count())));
    output.push('\n');

    if matches!(mode, ApplyHumanMode::Verbose) {
        let phases_heading = "Phases";
        output.push_str(&format!(
            "{}{}{}\n",
            color_header(),
            phases_heading,
            color_reset(),
        ));
        output.push_str(&format!("{}\n", "─".repeat(phases_heading.chars().count())));
        for phase in dedup_terminal_phase_states(&view.phases) {
            output.push_str(&format!(
                "{}\t{}\n",
                humane_phase_label(&phase.phase),
                phase_state_label(&phase.state)
            ));
        }
        output.push('\n');
    }

    let execution_heading = "Execution";
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        execution_heading,
        color_reset(),
    ));
    output.push_str(&format!(
        "{}\n",
        "─".repeat(execution_heading.chars().count())
    ));
    let visible_events = terminal_events
        .iter()
        .filter(|event| apply_event_visible(event, mode))
        .collect::<Vec<_>>();
    for (index, event) in visible_events.iter().enumerate() {
        let Some(entry) = entry_by_name.get(event.object.name.as_str()).copied() else {
            continue;
        };
        let has_details = render_apply_entry(
            &mut output,
            plan,
            entry,
            event,
            &entry_by_name,
            label_column,
            mode,
        );
        if has_details && index + 1 != visible_events.len() {
            output.push('\n');
        }
    }
    output.push('\n');

    let summary_heading = "Summary";
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        summary_heading,
        color_reset(),
    ));
    output.push_str(&format!(
        "{}\n",
        "─".repeat(summary_heading.chars().count())
    ));
    output.push_str(&format!(
        "{}\n",
        crate::cli::status::render_apply_summary(&view, convergence)
    ));
    output
}

impl ApplyProgressRenderer {
    pub fn new(
        plan: &DeterministicReconciliationPlan,
        mode: ApplyHumanMode,
        run_display_state: ApplyRunDisplayState,
    ) -> Self {
        let plan_view = build_plan_output(plan);
        let label_column = plan_view
            .entries
            .iter()
            .map(|entry| visible_width(&entry.object.display_id))
            .max()
            .unwrap_or(0)
            + 16;
        let entry_by_name = plan_view
            .entries
            .iter()
            .cloned()
            .map(|entry| (entry.object.name.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        Self {
            plan: plan.clone(),
            plan_view,
            entry_by_name,
            label_column,
            mode,
            run_display_state,
            streamed_started: BTreeSet::new(),
            streamed_terminal: BTreeSet::new(),
        }
    }

    pub fn begin(&self) -> String {
        let header = format!(
            "Apply for host {} @ {}",
            display_scope_name(&self.plan.scope_id),
            apply_header_revision_context(&self.plan_view.revision_context, self.run_display_state),
        );
        let execution_heading = "Execution";
        format!(
            "{}{}{}\n{}\n\n{}{}{}\n{}\n",
            color_header(),
            header,
            color_reset(),
            "─".repeat(header.chars().count()),
            color_header(),
            execution_heading,
            color_reset(),
            "─".repeat(execution_heading.chars().count()),
        )
    }

    pub fn begin_interactive(&self) -> ApplyInteractiveEvent {
        ApplyInteractiveEvent::Begin(self.begin())
    }

    pub fn render_started(&mut self, target: &str) -> Option<String> {
        let entry = self.entry_by_name.get(target)?;
        if !apply_entry_is_live(entry, self.mode) {
            return None;
        }
        if !self.streamed_started.insert(target.to_string()) {
            return None;
        }
        let mut output = format_plan_label_line(
            "",
            Some(colored_marker(&entry.action)),
            &entry.object.display_id,
            apply_progress_label(&entry.action),
            self.label_column,
        );
        if !entry.dependencies.is_empty() {
            output.push_str("    requires\n");
            for dependency in &entry.dependencies {
                output.push_str(&format!("      - {}\n", dependency.object.display_id));
            }
        }
        Some(output)
    }

    pub fn render_started_interactive(&mut self, target: &str) -> Option<ApplyInteractiveEvent> {
        let entry = self.entry_by_name.get(target)?;
        if !apply_entry_is_live(entry, self.mode) {
            return None;
        }
        if !self.streamed_started.insert(target.to_string()) {
            return None;
        }
        let line = format_plan_label_line(
            "",
            Some(colored_marker(&entry.action)),
            &entry.object.display_id,
            apply_progress_label(&entry.action),
            self.label_column,
        )
        .trim_end_matches('\n')
        .to_string();
        Some(ApplyInteractiveEvent::Started {
            target: target.to_string(),
            line,
        })
    }

    pub fn render_completed(&mut self, target: &str) -> Option<String> {
        let entry = self.entry_by_name.get(target)?;
        if !apply_entry_is_live(entry, self.mode) {
            return None;
        }
        if !self.streamed_terminal.insert(target.to_string()) {
            return None;
        }
        Some(format_plan_label_line(
            "",
            Some(colored_marker(&entry.action)),
            &entry.object.display_id,
            apply_terminal_label(&entry.action),
            self.label_column,
        ))
    }

    pub fn render_completed_interactive(&mut self, target: &str) -> Option<ApplyInteractiveEvent> {
        let block = self.render_completed(target)?;
        Some(ApplyInteractiveEvent::Terminal {
            target: target.to_string(),
            block,
        })
    }

    pub fn render_failed(&mut self, target: &str, error: &str) -> Option<String> {
        let entry = self.entry_by_name.get(target)?;
        if !self.streamed_terminal.insert(target.to_string()) {
            return None;
        }
        let mut output = format_plan_label_line(
            "",
            Some(apply_marker_for_failed_entry(entry)),
            &entry.object.display_id,
            "failed",
            self.label_column,
        );
        if let Some(message) = normalized_failure_message(error) {
            output.push_str(&format!("    {}\n", message));
        }
        output.push_str("    failed during Applying\n");
        let suggested = suggested_debug_commands_for_entry(entry);
        if !suggested.is_empty() {
            output.push_str("    suggested checks\n");
            for command in suggested {
                output.push_str(&format!("      - {}\n", command));
            }
        }
        Some(output)
    }

    pub fn render_failed_interactive(
        &mut self,
        target: &str,
        error: &str,
    ) -> Option<ApplyInteractiveEvent> {
        let block = self.render_failed(target, error)?;
        Some(ApplyInteractiveEvent::Terminal {
            target: target.to_string(),
            block,
        })
    }

    pub fn finish(
        &mut self,
        verification_results: &[VerificationResult],
        convergence: Option<&DeterministicConvergenceRecord>,
    ) -> String {
        let view = build_apply_output(&self.plan, verification_results, convergence);
        let entry_by_name = self
            .entry_by_name
            .iter()
            .map(|(name, entry)| (name.as_str(), entry))
            .collect::<BTreeMap<_, _>>();
        let mut output = String::new();
        for event in terminal_apply_events(&view) {
            if !apply_event_visible(&event, self.mode) {
                continue;
            }
            if self.streamed_terminal.contains(event.object.name.as_str())
                && matches!(
                    event.state,
                    ExecutionState::Created
                        | ExecutionState::Updated
                        | ExecutionState::Deleted
                        | ExecutionState::Recovered
                        | ExecutionState::Restarted
                )
            {
                continue;
            }
            let Some(entry) = entry_by_name.get(event.object.name.as_str()).copied() else {
                continue;
            };
            let has_details = render_apply_entry(
                &mut output,
                &self.plan,
                entry,
                &event,
                &entry_by_name,
                self.label_column,
                self.mode,
            );
            if has_details {
                output.push('\n');
            }
        }
        if !output.is_empty() {
            output.push('\n');
        }
        let summary_heading = "Summary";
        output.push_str(&format!(
            "{}{}{}\n",
            color_header(),
            summary_heading,
            color_reset(),
        ));
        output.push_str(&format!(
            "{}\n",
            "─".repeat(summary_heading.chars().count())
        ));
        output.push_str(&format!(
            "{}\n",
            crate::cli::status::render_apply_summary(&view, convergence)
        ));
        output
    }

    pub fn finish_interactive(
        &mut self,
        verification_results: &[VerificationResult],
        convergence: Option<&DeterministicConvergenceRecord>,
    ) -> ApplyInteractiveEvent {
        let mut text = self.finish(verification_results, convergence);
        if !text.is_empty() && !text.starts_with('\n') {
            text.insert(0, '\n');
        }
        ApplyInteractiveEvent::Finish(text)
    }
}

pub fn build_revision_context(plan: &DeterministicReconciliationPlan) -> RevisionContext {
    RevisionContext {
        target_revision: plan
            .desired_revision_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        requested_repository: plan.requested_repository.clone(),
        requested_ref: plan.requested_ref.clone(),
        scope_id: Some(plan.scope_id.clone()),
        last_applied_revision: plan.baseline_revision_id.clone(),
        last_applied_requested_repository: plan.last_applied_requested_repository.clone(),
        last_applied_requested_ref: plan.last_applied_requested_ref.clone(),
        change_revision: plan.desired_revision_id.clone(),
    }
}

fn build_apply_phase_events(
    convergence: Option<&DeterministicConvergenceRecord>,
) -> Vec<ApplyPhaseEvent> {
    let convergence_state = if convergence
        .map(|record| {
            matches!(
                record.status,
                crate::core::types::ConvergenceStatus::Success
            )
        })
        .unwrap_or(false)
    {
        PhaseState::Completed
    } else {
        PhaseState::Failed
    };
    let mut sequence = 0usize;
    let mut push = |phase: ApplyPhaseKind, state: PhaseState, phases: &mut Vec<ApplyPhaseEvent>| {
        phases.push(ApplyPhaseEvent {
            phase,
            state,
            sequence,
        });
        sequence += 1;
    };
    let mut phases = Vec::new();
    for phase in [
        ApplyPhaseKind::Resolution,
        ApplyPhaseKind::GraphConstruction,
        ApplyPhaseKind::Planning,
        ApplyPhaseKind::Execution,
    ] {
        push(phase.clone(), PhaseState::Started, &mut phases);
        push(phase, PhaseState::Completed, &mut phases);
    }
    push(
        ApplyPhaseKind::ConvergenceCheck,
        PhaseState::Started,
        &mut phases,
    );
    push(
        ApplyPhaseKind::ConvergenceCheck,
        convergence_state,
        &mut phases,
    );
    push(
        ApplyPhaseKind::FinalSummary,
        PhaseState::Started,
        &mut phases,
    );
    push(
        ApplyPhaseKind::FinalSummary,
        PhaseState::Completed,
        &mut phases,
    );
    phases
}

fn verification_failed_for_entry(
    entry: &PlanEntry,
    verification_results: &[VerificationResult],
) -> bool {
    verification_results.iter().any(|result| {
        result.target == entry.object.name
            && matches!(
                result.status,
                crate::core::types::VerificationStatus::Failure
            )
    })
}

fn convergence_failed_for_entry(
    entry: &PlanEntry,
    convergence: Option<&DeterministicConvergenceRecord>,
) -> bool {
    let Some(convergence) = convergence else {
        return false;
    };
    let runtime_unit = systemd_unit_for_quadlet_file(&entry.object.name);
    let display_id = entry.object.display_id.as_str();
    convergence.failed_actions.iter().any(|failed| {
        failed == &entry.object.name || failed == &runtime_unit || failed == display_id
    })
}

fn verification_result_for_entry<'a>(
    entry: &PlanEntry,
    verification_results: &'a [VerificationResult],
) -> Option<&'a VerificationResult> {
    verification_results.iter().find(|result| {
        result.target == entry.object.name
            && matches!(
                result.status,
                crate::core::types::VerificationStatus::Failure
            )
    })
}

fn failure_cause_for_entry(
    entry: &PlanEntry,
    verification_results: &[VerificationResult],
) -> Option<Cause> {
    let result = verification_result_for_entry(entry, verification_results)?;
    let summary = result
        .details
        .clone()
        .unwrap_or_else(|| "verification failed".to_string());
    let mut details = BTreeMap::new();
    details.insert("verification_target".to_string(), result.target.clone());
    if let Some(message) = &result.details {
        details.insert("verification_details".to_string(), message.clone());
    }
    Some(Cause {
        kind: CauseKind::RuntimeVariance,
        summary,
        source_object: None,
        details: Some(details),
    })
}

fn impacted_objects_for_entry(entry: &PlanEntry) -> Option<Vec<ManagedObjectRef>> {
    let impacted = entry
        .dependencies
        .iter()
        .map(|dependency| dependency.object.clone())
        .collect::<Vec<_>>();
    (!impacted.is_empty()).then_some(impacted)
}

fn dedup_terminal_phase_states(phases: &[ApplyPhaseEvent]) -> Vec<ApplyPhaseEvent> {
    let mut resolved = None;
    let mut graph = None;
    let mut planning = None;
    let mut execution = None;
    let mut convergence = None;
    let mut summary = None;
    for phase in phases {
        match phase.phase {
            ApplyPhaseKind::Resolution => resolved = Some(phase.clone()),
            ApplyPhaseKind::GraphConstruction => graph = Some(phase.clone()),
            ApplyPhaseKind::Planning => planning = Some(phase.clone()),
            ApplyPhaseKind::Execution => execution = Some(phase.clone()),
            ApplyPhaseKind::ConvergenceCheck => convergence = Some(phase.clone()),
            ApplyPhaseKind::FinalSummary => summary = Some(phase.clone()),
        }
    }
    [resolved, graph, planning, execution, convergence, summary]
        .into_iter()
        .flatten()
        .collect()
}

fn humane_phase_label(phase: &ApplyPhaseKind) -> &'static str {
    match phase {
        ApplyPhaseKind::Resolution => "Resolving desired state",
        ApplyPhaseKind::GraphConstruction => "Building dependency graph",
        ApplyPhaseKind::Planning => "Planning",
        ApplyPhaseKind::Execution => "Applying",
        ApplyPhaseKind::ConvergenceCheck => "Verifying",
        ApplyPhaseKind::FinalSummary => "Summarizing",
    }
}

fn result_final_state_for_event(event: &ExecutionEvent) -> ResultFinalState {
    match event.state {
        ExecutionState::Failed => ResultFinalState::Failed,
        ExecutionState::Blocked => ResultFinalState::Blocked,
        ExecutionState::Skipped => ResultFinalState::Skipped,
        ExecutionState::Unchanged => ResultFinalState::NoOp,
        ExecutionState::Created
        | ExecutionState::Updated
        | ExecutionState::Deleted
        | ExecutionState::Recovered
        | ExecutionState::Restarted => ResultFinalState::Succeeded,
        ExecutionState::Pending | ExecutionState::Running => ResultFinalState::Failed,
    }
}

fn result_outcome_for_convergence(
    convergence: Option<&DeterministicConvergenceRecord>,
) -> ResultOutcome {
    match convergence.map(|record| &record.status) {
        Some(crate::core::types::ConvergenceStatus::Success) => ResultOutcome::Converged,
        Some(crate::core::types::ConvergenceStatus::Partial) => ResultOutcome::PartiallyApplied,
        Some(crate::core::types::ConvergenceStatus::Blocked) => ResultOutcome::Failed,
        Some(crate::core::types::ConvergenceStatus::RepeatedFailure)
        | Some(crate::core::types::ConvergenceStatus::Oscillation) => ResultOutcome::NonConverging,
        Some(crate::core::types::ConvergenceStatus::Failed) | None => ResultOutcome::Failed,
    }
}

fn result_outcome_label(outcome: &ResultOutcome) -> &'static str {
    match outcome {
        ResultOutcome::Converged => "converged",
        ResultOutcome::ConvergedWithToleratedVariance => "converged with tolerated variance",
        ResultOutcome::PartiallyApplied => "partially applied",
        ResultOutcome::Failed => "failed",
        ResultOutcome::NonConverging => "non-converging",
    }
}

fn result_marker(entry: &ResultEntry) -> String {
    match entry.final_state {
        ResultFinalState::Succeeded => entry
            .action
            .as_ref()
            .map(colored_marker)
            .unwrap_or_else(|| colored_marker(&PlanEntryAction::NoOp)),
        ResultFinalState::Failed => format!("{}[!]{}", color_blocked(), color_reset()),
        ResultFinalState::Blocked => colored_marker(&PlanEntryAction::Blocked),
        ResultFinalState::Skipped => colored_marker(&PlanEntryAction::Skipped),
        ResultFinalState::NoOp => colored_marker(&PlanEntryAction::NoOp),
    }
}

fn result_label(entry: &ResultEntry) -> &'static str {
    match entry.final_state {
        ResultFinalState::Succeeded => match entry.action.as_ref() {
            Some(PlanEntryAction::Create) => "created",
            Some(PlanEntryAction::Update) => "updated",
            Some(PlanEntryAction::Delete) => "deleted",
            Some(PlanEntryAction::Recover) => "recovered",
            Some(PlanEntryAction::Restart) => "restarted",
            Some(PlanEntryAction::Replace) => "replaced",
            Some(PlanEntryAction::NoOp) | None => "unchanged",
            Some(PlanEntryAction::Blocked) => "blocked",
            Some(PlanEntryAction::Skipped) => "skipped",
        },
        ResultFinalState::Failed => "failed",
        ResultFinalState::Blocked => "blocked",
        ResultFinalState::Skipped => "skipped",
        ResultFinalState::NoOp => "unchanged",
    }
}

fn result_summary_line(summary: &ResultSummaryView) -> String {
    let parts = [
        summary_count_phrase(summary.changed_count, "change", "changes"),
        summary_count_phrase(summary.failed_count, "failed", "failed"),
        summary_count_phrase(summary.blocked_count, "blocked", "blocked"),
        summary_count_phrase(summary.skipped_count, "skipped", "skipped"),
        summary_count_phrase(summary.unchanged_count, "unchanged", "unchanged"),
    ];
    parts
        .into_iter()
        .filter(|part| !part.starts_with("0 "))
        .collect::<Vec<_>>()
        .join(" • ")
}

fn explain_action_or_outcome(entry: &ResultEntry) -> String {
    result_label(entry).to_string()
}

fn humane_cause_summary(cause: &Cause) -> String {
    match cause.kind {
        CauseKind::NoChange => "declarative state matches desired state".to_string(),
        _ => cause.summary.clone(),
    }
}

fn default_explain_reason(entry: &ResultEntry) -> String {
    match entry.action {
        Some(PlanEntryAction::NoOp) => "declarative state matches desired state".to_string(),
        Some(PlanEntryAction::Recover) => "runtime reconciliation is required".to_string(),
        Some(PlanEntryAction::Restart) => "a prerequisite change requires reactivation".to_string(),
        Some(PlanEntryAction::Create) => {
            "this object does not yet exist in the desired state".to_string()
        }
        Some(PlanEntryAction::Update | PlanEntryAction::Replace) => {
            "declarative state differs from the desired state".to_string()
        }
        Some(PlanEntryAction::Delete) => {
            "this object is no longer present in the desired state".to_string()
        }
        Some(PlanEntryAction::Blocked) => {
            "a blocking prerequisite prevents reconciliation".to_string()
        }
        Some(PlanEntryAction::Skipped) => {
            "an earlier failure prevented safe reconciliation".to_string()
        }
        None if matches!(entry.final_state, ResultFinalState::NoOp) => {
            "declarative state matches desired state".to_string()
        }
        None => "no additional explanation available".to_string(),
    }
}

fn default_explain_reason_from_view(view: &ExplainOutputView) -> String {
    match view.action_or_outcome.as_str() {
        "unchanged" => "declarative state matches desired state".to_string(),
        "recovered" | "recover" => "runtime reconciliation is required".to_string(),
        "restarted" => "a prerequisite change requires reactivation".to_string(),
        "created" => "this object does not yet exist in the desired state".to_string(),
        "updated" => "declarative state differs from the desired state".to_string(),
        "deleted" => "this object is no longer present in the desired state".to_string(),
        "blocked" => "a blocking prerequisite prevents reconciliation".to_string(),
        "skipped" => "an earlier failure prevented safe reconciliation".to_string(),
        _ => "no additional explanation available".to_string(),
    }
}

fn explain_humane_state_label(action_or_outcome: &str) -> &str {
    match action_or_outcome {
        "created" => "created",
        "updated" => "updated",
        "deleted" => "deleted",
        "recovered" => "recovered",
        "restarted" => "restarted",
        "failed" => "failed",
        "blocked" => "blocked",
        "skipped" => "skipped",
        _ => "unchanged",
    }
}

fn explain_runtime_line(view: &ExplainOutputView) -> Option<&'static str> {
    match view.action_or_outcome.as_str() {
        "unchanged" => Some("not evaluated in plan"),
        "recover" | "recovered" => Some("recovery required"),
        _ => None,
    }
}

fn format_explain_context(view: &ExplainOutputView) -> Option<String> {
    let mut lines = Vec::new();
    if let Some(scope_id) = view.revision_context.scope_id.as_deref() {
        lines.push(format!("Host:   {}", display_scope_name(scope_id)));
    }
    lines.push(format!(
        "Target: {}",
        crate::cli::status::render_revision_with_requested_ref(
            &view.revision_context.target_revision,
            view.revision_context.requested_ref.as_deref(),
        )
    ));
    if let Some(previous) = view.revision_context.last_applied_revision.as_deref() {
        lines.push(format!(
            "Last:   {}",
            crate::cli::status::render_previous_revision_with_requested_ref(
                previous,
                view.revision_context.last_applied_requested_ref.as_deref(),
            )
        ));
    }
    (!lines.is_empty()).then(|| format!("{}\n", lines.join("\n")))
}

fn explain_apply_intent(entry: &ResultEntry, has_convergence: bool) -> String {
    match (has_convergence, entry.action.as_ref(), &entry.final_state) {
        (_, Some(PlanEntryAction::NoOp), _) | (_, None, ResultFinalState::NoOp) => {
            "no action planned".to_string()
        }
        (false, Some(PlanEntryAction::Create), _) => {
            "CoreOps will create this object after its prerequisites are reconciled.".to_string()
        }
        (false, Some(PlanEntryAction::Update | PlanEntryAction::Replace), _) => {
            "CoreOps will update this object after its prerequisites are reconciled.".to_string()
        }
        (false, Some(PlanEntryAction::Delete), _) => {
            "CoreOps will delete this object after dependents are reconciled.".to_string()
        }
        (false, Some(PlanEntryAction::Restart), _) => {
            "CoreOps will restart this object after its prerequisites are reconciled.".to_string()
        }
        (false, Some(PlanEntryAction::Recover), _) => {
            "CoreOps will attempt runtime recovery for this object.".to_string()
        }
        (false, Some(PlanEntryAction::Blocked), _) => {
            "CoreOps cannot act until the blocking prerequisite is resolved.".to_string()
        }
        (false, Some(PlanEntryAction::Skipped), _) => {
            "CoreOps will not act on this object in the current plan.".to_string()
        }
        (true, _, ResultFinalState::Succeeded) => {
            format!(
                "CoreOps applied this object and reported it as {}.",
                result_label(entry)
            )
        }
        (true, _, ResultFinalState::Failed) => {
            "CoreOps attempted reconciliation, but this object did not converge.".to_string()
        }
        (true, _, ResultFinalState::Blocked) => {
            "CoreOps could not act on this object because a prerequisite blocked progress."
                .to_string()
        }
        (true, _, ResultFinalState::Skipped) => {
            "CoreOps skipped this object because an earlier failure prevented safe progress."
                .to_string()
        }
        (true, _, ResultFinalState::NoOp) => {
            "no action was needed in the last apply run".to_string()
        }
        (
            false,
            None,
            ResultFinalState::Succeeded
            | ResultFinalState::Failed
            | ResultFinalState::Blocked
            | ResultFinalState::Skipped,
        ) => "CoreOps has no planned action recorded for this object.".to_string(),
    }
}

fn explain_summary(entry: &ResultEntry, has_convergence: bool) -> String {
    let object = &entry.object.display_id;
    match (has_convergence, entry.final_state.clone()) {
        (_, ResultFinalState::NoOp) => {
            format!("{object} is unchanged and has no planned reconciliation action.")
        }
        (false, _) => format!(
            "{object} is {} and {}",
            result_label(entry),
            explain_apply_intent(entry, false).trim_end_matches('.')
        ),
        (true, ResultFinalState::Succeeded) => {
            format!("{object} completed with {} state.", result_label(entry))
        }
        (true, ResultFinalState::Failed) => {
            format!("{object} did not converge in the last apply run.")
        }
        (true, ResultFinalState::Blocked) => {
            format!("{object} remained blocked in the last apply run.")
        }
        (true, ResultFinalState::Skipped) => {
            format!("{object} was skipped in the last apply run.")
        }
    }
}

fn explain_metadata(entry: &ResultEntry) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert(
        "resource_type".to_string(),
        entry.object.resource_type.clone(),
    );
    metadata.insert("managed_name".to_string(), entry.object.name.clone());
    metadata.insert(
        "runtime_unit".to_string(),
        systemd_unit_for_quadlet_file(&entry.object.name),
    );
    if entry.object.name.ends_with(".mount") {
        metadata.insert(
            "automount_unit".to_string(),
            entry.object.name.replace(".mount", ".automount"),
        );
    }
    metadata
}

fn explain_x_coreops(entry: &ResultEntry) -> Option<BTreeMap<String, serde_json::Value>> {
    if entry.object.name.ends_with(".mount") || entry.object.name.ends_with(".automount") {
        let mut map = BTreeMap::new();
        map.insert(
            "CreateMountpoint".to_string(),
            serde_json::Value::Bool(true),
        );
        return Some(map);
    }
    None
}

fn json_value_to_humane(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn display_scope_name_from_revision_context(revision_context: &RevisionContext) -> &str {
    revision_context
        .scope_id
        .as_deref()
        .map(display_scope_name)
        .unwrap_or("current")
}

fn result_header_revision_context(revision_context: &RevisionContext) -> String {
    match revision_context.last_applied_revision.as_deref() {
        Some(previous) if previous != revision_context.target_revision => format!(
            "{} → {}",
            crate::cli::status::render_previous_revision_with_requested_ref(
                previous,
                revision_context.last_applied_requested_ref.as_deref(),
            ),
            crate::cli::status::render_revision_with_requested_ref(
                &revision_context.target_revision,
                revision_context.requested_ref.as_deref(),
            )
        ),
        _ => crate::cli::status::render_revision_with_requested_ref(
            &revision_context.target_revision,
            revision_context.requested_ref.as_deref(),
        ),
    }
}

fn execution_state_label(state: &ExecutionState) -> &'static str {
    match state {
        ExecutionState::Pending => "pending",
        ExecutionState::Running => "running",
        ExecutionState::Created => "created",
        ExecutionState::Updated => "updated",
        ExecutionState::Deleted => "deleted",
        ExecutionState::Recovered => "recovered",
        ExecutionState::Restarted => "restarted",
        ExecutionState::Unchanged => "unchanged",
        ExecutionState::Blocked => "blocked",
        ExecutionState::Failed => "failed",
        ExecutionState::Skipped => "skipped",
    }
}

fn colored_marker_from_execution_state(state: &ExecutionState) -> String {
    match state {
        ExecutionState::Created => colored_marker(&PlanEntryAction::Create),
        ExecutionState::Updated => colored_marker(&PlanEntryAction::Update),
        ExecutionState::Deleted => colored_marker(&PlanEntryAction::Delete),
        ExecutionState::Recovered => colored_marker(&PlanEntryAction::Recover),
        ExecutionState::Restarted => colored_marker(&PlanEntryAction::Restart),
        ExecutionState::Unchanged => colored_marker(&PlanEntryAction::NoOp),
        ExecutionState::Blocked => colored_marker(&PlanEntryAction::Blocked),
        ExecutionState::Failed => format!("{}[!]{}", color_blocked(), color_reset()),
        ExecutionState::Skipped => colored_marker(&PlanEntryAction::Skipped),
        ExecutionState::Pending | ExecutionState::Running => colored_marker(&PlanEntryAction::NoOp),
    }
}

fn apply_outcome_label_from_view(view: &ApplyOutputView) -> &'static str {
    let convergence_state = view
        .phases
        .iter()
        .rev()
        .find(|phase| phase.phase == ApplyPhaseKind::ConvergenceCheck)
        .map(|phase| phase.state.clone());
    match convergence_state {
        Some(PhaseState::Completed) => "converged",
        Some(PhaseState::Failed) => "non-converging",
        _ => "unknown",
    }
}

fn convergence_outcome_label(status: &crate::core::types::ConvergenceStatus) -> &'static str {
    match status {
        crate::core::types::ConvergenceStatus::Success => "converged",
        crate::core::types::ConvergenceStatus::Partial => "partially applied",
        crate::core::types::ConvergenceStatus::Blocked => "blocked",
        crate::core::types::ConvergenceStatus::RepeatedFailure
        | crate::core::types::ConvergenceStatus::Oscillation => "non-converging",
        crate::core::types::ConvergenceStatus::Failed => "failed",
    }
}

fn phase_state_label(state: &PhaseState) -> &'static str {
    match state {
        PhaseState::Started => "started",
        PhaseState::Completed => "completed",
        PhaseState::Failed => "failed",
    }
}

fn terminal_execution_state(action: &PlanEntryAction) -> ExecutionState {
    match action {
        PlanEntryAction::Create => ExecutionState::Created,
        PlanEntryAction::Update | PlanEntryAction::Replace => ExecutionState::Updated,
        PlanEntryAction::Delete => ExecutionState::Deleted,
        PlanEntryAction::Recover => ExecutionState::Recovered,
        PlanEntryAction::Restart => ExecutionState::Restarted,
        PlanEntryAction::NoOp => ExecutionState::Unchanged,
        PlanEntryAction::Blocked => ExecutionState::Blocked,
        PlanEntryAction::Skipped => ExecutionState::Skipped,
    }
}

fn apply_event_short_label(event: &ExecutionEvent) -> &'static str {
    match event.state {
        ExecutionState::Created => "created",
        ExecutionState::Updated => "updated",
        ExecutionState::Deleted => "deleted",
        ExecutionState::Recovered => "recovered",
        ExecutionState::Restarted => "restarted",
        ExecutionState::Unchanged => "unchanged",
        ExecutionState::Failed => "failed",
        ExecutionState::Blocked => "blocked",
        ExecutionState::Skipped => "skipped",
        ExecutionState::Pending => "pending",
        ExecutionState::Running => "running",
    }
}

fn apply_progress_label(action: &PlanEntryAction) -> &'static str {
    match action {
        PlanEntryAction::Create => "creating...",
        PlanEntryAction::Update | PlanEntryAction::Replace => "updating...",
        PlanEntryAction::Delete => "deleting...",
        PlanEntryAction::Recover => "recovering...",
        PlanEntryAction::Restart => "restarting...",
        PlanEntryAction::NoOp => "checking...",
        PlanEntryAction::Blocked => "blocked",
        PlanEntryAction::Skipped => "skipped",
    }
}

fn apply_terminal_label(action: &PlanEntryAction) -> &'static str {
    match action {
        PlanEntryAction::Create => "created",
        PlanEntryAction::Update | PlanEntryAction::Replace => "updated",
        PlanEntryAction::Delete => "deleted",
        PlanEntryAction::Recover => "recovered",
        PlanEntryAction::Restart => "restarted",
        PlanEntryAction::NoOp => "unchanged",
        PlanEntryAction::Blocked => "blocked",
        PlanEntryAction::Skipped => "skipped",
    }
}

fn apply_entry_is_live(entry: &PlanEntry, mode: ApplyHumanMode) -> bool {
    match mode {
        ApplyHumanMode::Verbose => !matches!(
            entry.action,
            PlanEntryAction::Blocked | PlanEntryAction::Skipped | PlanEntryAction::NoOp
        ),
        ApplyHumanMode::Default => matches!(
            entry.action,
            PlanEntryAction::Create
                | PlanEntryAction::Update
                | PlanEntryAction::Replace
                | PlanEntryAction::Delete
                | PlanEntryAction::Recover
                | PlanEntryAction::Restart
        ),
    }
}

fn apply_event_visible(event: &ExecutionEvent, mode: ApplyHumanMode) -> bool {
    match mode {
        ApplyHumanMode::Verbose => true,
        ApplyHumanMode::Default => !matches!(
            event.state,
            ExecutionState::Unchanged | ExecutionState::Skipped
        ),
    }
}

fn terminal_apply_events(view: &ApplyOutputView) -> Vec<ExecutionEvent> {
    view.events
        .iter()
        .filter(|event| {
            matches!(
                event.event_kind,
                ExecutionEventKind::ObjectTerminal | ExecutionEventKind::ObjectSkipped
            )
        })
        .cloned()
        .collect()
}

fn apply_marker_for_failed_entry(entry: &PlanEntry) -> String {
    match entry.action {
        PlanEntryAction::Create
        | PlanEntryAction::Update
        | PlanEntryAction::Replace
        | PlanEntryAction::Delete
        | PlanEntryAction::Recover
        | PlanEntryAction::Restart
        | PlanEntryAction::Blocked => colored_marker(&PlanEntryAction::Blocked),
        PlanEntryAction::NoOp | PlanEntryAction::Skipped => {
            colored_marker(&PlanEntryAction::Blocked)
        }
    }
}

fn apply_header_revision_context(
    revision_context: &RevisionContext,
    run_display_state: ApplyRunDisplayState,
) -> String {
    let previous = revision_context.last_applied_revision.as_deref();
    let target = revision_context.target_revision.as_str();
    let requested_ref = revision_context.requested_ref.as_deref();
    match run_display_state {
        ApplyRunDisplayState::FirstRun => format!(
            "{} (first run)",
            crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
        ),
        ApplyRunDisplayState::Recovery => {
            format!(
                "{} (recovery from failed initial apply)",
                crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
            )
        }
        ApplyRunDisplayState::Managed => match previous {
            Some(previous) if previous != target => {
                format!(
                    "{} → {}",
                    crate::cli::status::render_previous_revision_with_requested_ref(
                        previous,
                        revision_context.last_applied_requested_ref.as_deref(),
                    ),
                    crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
                )
            }
            _ => crate::cli::status::render_revision_with_requested_ref(target, requested_ref),
        },
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

fn significant_diff_fields(action: &crate::core::types::DeterministicPlannedAction) -> Vec<String> {
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
    plan.drift_records
        .iter()
        .find(|record| record.object_id == object_id)
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
            kind: match record.category {
                crate::core::types::DriftCategory::RuntimeVariance => CauseKind::RuntimeVariance,
                _ => CauseKind::Drift,
            },
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
            DeterministicActionClass::Recover => CauseKind::RecoveryRequired,
            DeterministicActionClass::Restart => CauseKind::DependencyChange,
            DeterministicActionClass::Blocked => CauseKind::BlockedPrerequisite,
            DeterministicActionClass::NoOp => CauseKind::NoChange,
        };
        if !matches!(action.classification, DeterministicActionClass::NoOp) {
            let source_object = match action.classification {
                DeterministicActionClass::Restart => restart_trigger_object(plan, action),
                _ => None,
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
    if let Some(action) = plan
        .actions
        .iter()
        .find(|action| action.object_id == object_id)
    {
        let direct_ids = direct_prerequisite_refs(&plan.graph, object_id)
            .into_iter()
            .map(|object| object.name)
            .collect::<BTreeSet<_>>();
        let object_kinds = object_kind_by_id(&plan.graph);
        edges.extend(
            action
                .dependency_context
                .iter()
                .filter(|dependency| !direct_ids.contains(*dependency))
                .map(|dependency| DependencyEdgeView {
                    relation: DependencyRelation::Blocker,
                    object: object_kinds
                        .get(dependency.as_str())
                        .map(|kind| managed_object_ref(dependency, kind))
                        .unwrap_or_else(|| inferred_managed_object_ref(dependency)),
                }),
        );
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
        DeterministicActionClass::Recover => PlanEntryAction::Recover,
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
                !matches!(
                    entry.action,
                    PlanEntryAction::NoOp | PlanEntryAction::Blocked | PlanEntryAction::Skipped
                )
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
    format_deterministic_plan_report_with_options_and_state(
        plan,
        false,
        if plan.baseline_revision_id.is_some() {
            ApplyRunDisplayState::Managed
        } else {
            ApplyRunDisplayState::FirstRun
        },
    )
}

pub fn format_deterministic_plan_report_with_options(
    plan: &DeterministicReconciliationPlan,
    verbose: bool,
) -> String {
    format_deterministic_plan_report_with_options_and_state(
        plan,
        verbose,
        if plan.baseline_revision_id.is_some() {
            ApplyRunDisplayState::Managed
        } else {
            ApplyRunDisplayState::FirstRun
        },
    )
}

pub fn format_deterministic_plan_report_with_options_and_state(
    plan: &DeterministicReconciliationPlan,
    verbose: bool,
    run_display_state: ApplyRunDisplayState,
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
        plan_header_revision_context_with_state(
            &view.revision_context,
            has_attention_drift(plan),
            run_display_state,
        ),
    );
    let mut output = String::new();
    output.push_str(&format!("{}{}{}\n", color_header(), header, color_reset(),));
    output.push_str(&format!("{}\n", "─".repeat(header.chars().count())));
    let context = RenderGroupContext {
        plan,
        entries: &view.entries,
        entry_by_name: &entry_by_name,
        label_column,
        verbose,
    };
    render_group(&mut output, &context, PlanEntryAction::Create, "Create");
    render_group(&mut output, &context, PlanEntryAction::Update, "Update");
    render_group(&mut output, &context, PlanEntryAction::Recover, "Recover");
    render_group(&mut output, &context, PlanEntryAction::Restart, "Restart");
    render_group(&mut output, &context, PlanEntryAction::Blocked, "Blocked");
    render_group(&mut output, &context, PlanEntryAction::Delete, "Delete");
    render_group(&mut output, &context, PlanEntryAction::NoOp, "Unchanged");
    let summary_heading = "Summary";
    output.push_str(&format!(
        "{}{}{}\n",
        color_header(),
        summary_heading,
        color_reset(),
    ));
    output.push_str(&format!(
        "{}\n",
        "─".repeat(summary_heading.chars().count())
    ));
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
        format!(
            "previous={} desired={}",
            compact_value(&actual),
            compact_value(&desired)
        )
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
    if let Some(action) = plan
        .actions
        .iter()
        .find(|action| action.object_id == object_id)
    {
        let direct_ids = children
            .iter()
            .map(|(_, object)| object.name.clone())
            .collect::<BTreeSet<_>>();
        children.extend(
            action
                .dependency_context
                .iter()
                .filter(|dependency| !direct_ids.contains(*dependency))
                .map(|dependency| {
                    (
                        DependencyRelation::Blocker,
                        inferred_managed_object_ref(dependency),
                    )
                }),
        );
    }
    children.sort_by(|a, b| a.1.display_id.cmp(&b.1.display_id));
    for (index, (relation, object)) in children.iter().enumerate() {
        let branch = if index + 1 == children.len() {
            "└─"
        } else {
            "├─"
        };
        let entry = entry_by_name.get(object.name.as_str()).copied();
        output.push_str(&format_plan_label_line(
            &format!("{}{} ", indent, branch),
            entry
                .map(|entry| colored_marker(&entry.action))
                .or_else(|| Some(colored_marker_for_relation(relation))),
            &object.display_id,
            entry
                .map(short_label)
                .unwrap_or_else(|| relation_label(relation)),
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
        PlanEntryAction::Update => "update",
        PlanEntryAction::Replace => "replace",
        PlanEntryAction::Delete => "orphaned",
        PlanEntryAction::Recover => "recover",
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
        PlanEntryAction::Recover => Some(cause.summary.clone()),
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
        PlanEntryAction::Recover => (color_restart(), "↺"),
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
    entries
        .iter()
        .filter(|entry| entry.action == action)
        .count()
}

fn summary_count_phrase(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

fn plan_header_revision_context_with_state(
    revision_context: &RevisionContext,
    has_drift: bool,
    run_display_state: ApplyRunDisplayState,
) -> String {
    let previous = revision_context.last_applied_revision.as_deref();
    let target = revision_context.target_revision.as_str();
    let requested_ref = revision_context.requested_ref.as_deref();
    match (previous, run_display_state) {
        (None, ApplyRunDisplayState::FirstRun) => format!(
            "{} (first run)",
            crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
        ),
        (None, ApplyRunDisplayState::Recovery) => format!(
            "{} (recovery from failed initial apply)",
            crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
        ),
        (None, ApplyRunDisplayState::Managed) => {
            crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
        }
        (Some(previous), _) if previous == target => {
            if has_drift {
                format!(
                    "{} (with drift)",
                    crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
                )
            } else {
                crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
            }
        }
        (Some(previous), _) => {
            if has_drift {
                format!(
                    "{} → {} (with drift)",
                    crate::cli::status::render_previous_revision_with_requested_ref(
                        previous,
                        revision_context.last_applied_requested_ref.as_deref(),
                    ),
                    crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
                )
            } else {
                format!(
                    "{} → {}",
                    crate::cli::status::render_previous_revision_with_requested_ref(
                        previous,
                        revision_context.last_applied_requested_ref.as_deref(),
                    ),
                    crate::cli::status::render_revision_with_requested_ref(target, requested_ref)
                )
            }
        }
    }
}

fn has_attention_drift(plan: &DeterministicReconciliationPlan) -> bool {
    plan.drift_records.iter().any(|record| {
        matches!(
            record.category,
            crate::core::types::DriftCategory::ExternalDrift
                | crate::core::types::DriftCategory::StaleResidue
        )
    })
}

fn plan_summary_line(
    entries: &[PlanEntry],
    blocked_count: usize,
    unchanged_count: usize,
    verbose: bool,
) -> String {
    let view = PlanOutputView {
        view_kind: "plan".to_string(),
        revision_context: RevisionContext {
            target_revision: "unknown".to_string(),
            requested_repository: None,
            requested_ref: None,
            scope_id: None,
            last_applied_revision: None,
            last_applied_requested_repository: None,
            last_applied_requested_ref: None,
            change_revision: None,
        },
        summary: PlanSummaryView {
            changed_count: entries
                .iter()
                .filter(|entry| {
                    !matches!(
                        entry.action,
                        PlanEntryAction::NoOp | PlanEntryAction::Blocked | PlanEntryAction::Skipped
                    )
                })
                .count(),
            unchanged_count,
            blocked_count,
            skipped_count: count_for_action(entries, PlanEntryAction::Skipped),
            total_count: Some(entries.len()),
        },
        entries: entries.to_vec(),
    };
    crate::cli::status::render_plan_count_summary(&view, verbose)
}

fn display_scope_name(scope_id: &str) -> &str {
    scope_id.strip_prefix("host:").unwrap_or(scope_id)
}

fn color_reset() -> &'static str {
    "\x1b[0m"
}
fn color_header() -> &'static str {
    "\x1b[1m\x1b[37m"
}
fn color_create() -> &'static str {
    "\x1b[32m"
}
fn color_update() -> &'static str {
    "\x1b[33m"
}
fn color_delete() -> &'static str {
    "\x1b[31m"
}
fn color_restart() -> &'static str {
    "\x1b[34m"
}
fn color_blocked() -> &'static str {
    "\x1b[35m"
}
fn color_noop() -> &'static str {
    "\x1b[2m\x1b[37m"
}

const DEFAULT_DIFF_CONTEXT_LINES: usize = 6;

struct RenderGroupContext<'a> {
    plan: &'a DeterministicReconciliationPlan,
    entries: &'a [PlanEntry],
    entry_by_name: &'a BTreeMap<&'a str, &'a PlanEntry>,
    label_column: usize,
    verbose: bool,
}

fn render_group(
    output: &mut String,
    context: &RenderGroupContext<'_>,
    action: PlanEntryAction,
    title: &str,
) {
    let group = context
        .entries
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
    for (index, entry) in group.iter().enumerate() {
        let has_details = render_entry(
            output,
            context.plan,
            entry,
            context.entry_by_name,
            context.label_column,
            context.verbose,
        );
        if has_details && index + 1 != group.len() {
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
    if should_render_plan_dependencies(entry, entry_by_name, verbose)
        && !entry.dependencies.is_empty()
    {
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

fn should_render_plan_dependencies(
    entry: &PlanEntry,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
    verbose: bool,
) -> bool {
    if verbose {
        return true;
    }
    if matches!(entry.action, PlanEntryAction::Recover) {
        return false;
    }

    has_explanatory_dependencies(entry, entry_by_name)
}

fn render_apply_entry(
    output: &mut String,
    plan: &DeterministicReconciliationPlan,
    entry: &PlanEntry,
    event: &ExecutionEvent,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
    label_column: usize,
    mode: ApplyHumanMode,
) -> bool {
    output.push_str(&format_plan_label_line(
        "",
        Some(apply_marker(event)),
        &entry.object.display_id,
        apply_event_short_label(event),
        label_column,
    ));

    let mut has_details = false;
    if let Some(line) = apply_cause_line(entry, event) {
        output.push_str(&format!("    {}\n", line));
        has_details = true;
    }
    if should_render_apply_dependencies(entry, event, entry_by_name)
        && !entry.dependencies.is_empty()
    {
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
    if should_render_apply_diff(event) {
        if let Some(diff) = &entry.diff {
            output.push_str(&format!(
                "    Δ {}\n",
                displayed_diff_summary(diff, matches!(mode, ApplyHumanMode::Verbose))
            ));
            render_diff_body(output, diff, matches!(mode, ApplyHumanMode::Verbose));
            has_details = true;
        }
    }
    if matches!(event.state, ExecutionState::Failed) {
        if let Some(phase) = &event.phase {
            output.push_str(&format!(
                "    failed during {}\n",
                humane_phase_label(phase)
            ));
            has_details = true;
        }
        let suggested = suggested_debug_commands(entry, event);
        if !suggested.is_empty() {
            output.push_str("    suggested checks\n");
            for command in suggested {
                output.push_str(&format!("      - {}\n", command));
            }
            has_details = true;
        }
    }
    has_details
}

fn apply_marker(event: &ExecutionEvent) -> String {
    match event.state {
        ExecutionState::Created => colored_marker(&PlanEntryAction::Create),
        ExecutionState::Updated => colored_marker(&PlanEntryAction::Update),
        ExecutionState::Deleted => colored_marker(&PlanEntryAction::Delete),
        ExecutionState::Recovered => colored_marker(&PlanEntryAction::Recover),
        ExecutionState::Restarted => colored_marker(&PlanEntryAction::Restart),
        ExecutionState::Unchanged | ExecutionState::Skipped => {
            colored_marker(&PlanEntryAction::NoOp)
        }
        ExecutionState::Blocked | ExecutionState::Failed => {
            colored_marker(&PlanEntryAction::Blocked)
        }
        ExecutionState::Pending | ExecutionState::Running => colored_marker(&PlanEntryAction::NoOp),
    }
}

fn apply_cause_line(entry: &PlanEntry, event: &ExecutionEvent) -> Option<String> {
    match event.state {
        ExecutionState::Created
        | ExecutionState::Updated
        | ExecutionState::Deleted
        | ExecutionState::Recovered
        | ExecutionState::Restarted => entry
            .causes
            .first()
            .map(|cause| format!("because {}", cause.summary)),
        ExecutionState::Blocked => entry
            .causes
            .first()
            .map(|cause| format!("because {}", cause.summary)),
        ExecutionState::Failed => event
            .cause
            .as_ref()
            .or_else(|| entry.causes.first())
            .map(|cause| cause.summary.clone()),
        ExecutionState::Unchanged
        | ExecutionState::Skipped
        | ExecutionState::Pending
        | ExecutionState::Running => None,
    }
}

fn should_render_apply_dependencies(
    entry: &PlanEntry,
    event: &ExecutionEvent,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
) -> bool {
    if matches!(entry.action, PlanEntryAction::Recover) {
        return false;
    }
    matches!(
        event.state,
        ExecutionState::Created
            | ExecutionState::Updated
            | ExecutionState::Deleted
            | ExecutionState::Recovered
            | ExecutionState::Restarted
            | ExecutionState::Blocked
            | ExecutionState::Failed
    ) && has_explanatory_dependencies(entry, entry_by_name)
}

fn has_explanatory_dependencies(
    entry: &PlanEntry,
    entry_by_name: &BTreeMap<&str, &PlanEntry>,
) -> bool {
    entry.dependencies.iter().any(|dependency| {
        if matches!(dependency.relation, DependencyRelation::Blocker) {
            return true;
        }
        let candidate = entry_by_name
            .get(dependency.object.name.as_str())
            .copied()
            .or_else(|| {
                entry_by_name
                    .values()
                    .copied()
                    .find(|candidate| candidate.object.display_id == dependency.object.display_id)
            });
        match candidate {
            Some(candidate) => !matches!(candidate.action, PlanEntryAction::NoOp),
            None => true,
        }
    })
}

fn normalized_failure_message(message: &str) -> Option<String> {
    let trimmed = message.trim();
    if trimmed.is_empty()
        || trimmed == "systemd command failed:"
        || trimmed == "systemd command failed"
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn should_render_apply_diff(event: &ExecutionEvent) -> bool {
    matches!(
        event.state,
        ExecutionState::Created
            | ExecutionState::Updated
            | ExecutionState::Deleted
            | ExecutionState::Failed
    )
}

fn suggested_debug_commands(entry: &PlanEntry, event: &ExecutionEvent) -> Vec<String> {
    if !matches!(
        event.state,
        ExecutionState::Failed | ExecutionState::Blocked
    ) {
        return Vec::new();
    }
    suggested_debug_commands_for_entry(entry)
}

fn suggested_debug_commands_for_entry(entry: &PlanEntry) -> Vec<String> {
    let mut commands = Vec::new();
    commands.push(format!("core-ops explain {}", entry.object.display_id));
    let runtime_unit = systemd_unit_for_quadlet_file(&entry.object.name);
    commands.push(format!("systemctl status {}", runtime_unit));
    commands.push(format!("journalctl -u {} -b", runtime_unit));
    if entry.object.resource_type == "mount" {
        if let Some(where_path) = mount_target_path_from_unit(&entry.object.name) {
            commands.push(format!("findmnt {}", where_path));
        }
    }
    commands
}

fn mount_target_path_from_unit(unit_name: &str) -> Option<String> {
    let stem = unit_name
        .strip_suffix(".mount")
        .or_else(|| unit_name.strip_suffix(".automount"))?;
    let mut path = String::from("/");
    let mut chars = stem.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '-' {
            if chars.peek() == Some(&'-') {
                chars.next();
                path.push('-');
            } else if !path.ends_with('/') {
                path.push('/');
            }
        } else {
            path.push(ch);
        }
    }
    Some(path)
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
            output.push_str(&format!(
                "      {}: {}\n",
                field,
                compact_diff_details(details)
            ));
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
    DiffHunkStats {
        additions,
        deletions,
    }
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
    if let Some(action) = plan
        .actions
        .iter()
        .find(|action| action.object_id == object_id)
    {
        let direct_ids = children
            .iter()
            .map(|(_, object)| object.name.clone())
            .collect::<BTreeSet<_>>();
        children.extend(
            action
                .dependency_context
                .iter()
                .filter(|dependency| !direct_ids.contains(*dependency))
                .map(|dependency| {
                    (
                        DependencyRelation::Blocker,
                        inferred_managed_object_ref(dependency),
                    )
                }),
        );
    }
    children.sort_by(|a, b| a.1.display_id.cmp(&b.1.display_id));
    for (index, (_, object)) in children.iter().enumerate() {
        let branch = if index + 1 == children.len() {
            "└─"
        } else {
            "├─"
        };
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

pub fn format_rollback_report_json(
    target: &RollbackTargetCandidate,
    plan: &DeterministicReconciliationPlan,
) -> String {
    serde_json::json!({
        "view_kind": "rollback_plan",
        "target": {
            "target_revision_id": target.target_revision_id,
            "eligibility": format!("{:?}", target.eligibility).to_lowercase(),
            "reason": target.reason,
            "scope_id": target.scope_id,
        },
        "plan": build_plan_output(plan),
    })
    .to_string()
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

fn dependency_relation_heading(relation: &DependencyRelation) -> &'static str {
    match relation {
        DependencyRelation::Prerequisite => "Requires",
        DependencyRelation::Dependent => "Used by",
        DependencyRelation::Blocker => "Blocked by",
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

fn verification_mode_label(mode: VerificationRunMode) -> &'static str {
    match mode {
        VerificationRunMode::Local => "local",
        VerificationRunMode::Ci => "ci",
        VerificationRunMode::Debug => "debug",
    }
}

fn verification_outcome_label(outcome: VerificationRunOutcome) -> &'static str {
    match outcome {
        VerificationRunOutcome::Passed => "passed",
        VerificationRunOutcome::AssertionFailure => "assertion_failure",
        VerificationRunOutcome::InfrastructureFailure => "infrastructure_failure",
        VerificationRunOutcome::Timeout => "timeout",
        VerificationRunOutcome::HarnessError => "harness_error",
    }
}

fn format_scenario_classes(classes: &[VerificationScenarioClass]) -> String {
    if classes.is_empty() {
        return "none".to_string();
    }

    classes
        .iter()
        .map(|class| match class {
            VerificationScenarioClass::Convergence => "convergence",
            VerificationScenarioClass::DriftCorrection => "drift_correction",
            VerificationScenarioClass::Idempotency => "idempotency",
            VerificationScenarioClass::UpgradeTransition => "upgrade_transition",
            VerificationScenarioClass::RebootResilience => "reboot_resilience",
            VerificationScenarioClass::ExplainApplyConsistency => "explain_apply_consistency",
            VerificationScenarioClass::RegressionDetection => "regression_detection",
            VerificationScenarioClass::ReleaseGateSuccess => "release_gate_success",
            VerificationScenarioClass::ReleaseGateFailure => "release_gate_failure",
            VerificationScenarioClass::VerificationEnvironmentIdentity => {
                "verification_environment_identity"
            }
            VerificationScenarioClass::VersionIdentityVisibility => "version_identity_visibility",
            VerificationScenarioClass::InstallationPathValidation => {
                "installation_path_validation"
            }
            VerificationScenarioClass::OperatorVerificationFlow => "operator_verification_flow",
            VerificationScenarioClass::OperatorVerificationReproducibility => {
                "operator_verification_reproducibility"
            }
            VerificationScenarioClass::ColdStartDistributionValidation => {
                "cold_start_distribution_validation"
            }
            VerificationScenarioClass::DistributionArtifactValidation => {
                "distribution_artifact_validation"
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
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
