use std::path::{Path, PathBuf};

use crate::cli::report::build_plan_output;
use crate::core::types::{
    ApplyOutputView, DesiredState, DeterministicConvergenceRecord, DeterministicPersistedState,
    DeterministicReconciliationPlan, PlanEntry, PlanEntryAction, PlanOutputView,
    VerificationResult, VerificationStatus,
};
use crate::core::errors::StateError;
use crate::core::types::ReconciliationStatus;
use crate::io::state::{
    parse_persisted_state_text, read_deterministic_state, read_persisted_state, resolve_state_file,
    DETERMINISTIC_STATE_FILE_NAME,
};

pub fn render_status_from_path(path: &Path) -> String {
    let base = match read_persisted_state(path) {
        Ok(Some(state)) => render_present_state(&state),
        Ok(None) => uninitialized_status(),
        Err(StateError::Corrupt(corrupt_path)) => corrupt_status(&corrupt_path),
        Err(_) => uninitialized_status(),
    };
    append_deterministic_status(base, path)
}

pub fn render_status(explicit: Option<PathBuf>) -> String {
    let path = resolve_state_file(explicit);
    render_status_from_path(&path)
}

pub fn format_status_text(contents: &str) -> String {
    match parse_persisted_state_text(contents) {
        Some(state) => render_present_state(&state),
        None => uninitialized_status(),
    }
}

fn render_present_state(state: &crate::core::types::PersistedProvenanceState) -> String {
    let lifecycle = derive_lifecycle_state(state);
    match serde_json::to_string_pretty(state) {
        Ok(pretty) => format!("lifecycle_state: {lifecycle}\nprovenance\n{}", pretty),
        Err(_) => uninitialized_status(),
    }
}

fn uninitialized_status() -> String {
    "lifecycle_state: Uninitialized\nprovenance\n{\n  \"status\": \"absent\"\n}".to_string()
}

fn corrupt_status(path: &str) -> String {
    format!(
        "lifecycle_state: Corrupt\nprovenance\n{{\n  \"status\": \"corrupt\",\n  \"path\": \"{}\",\n  \"hint\": \"run 'core-ops init --force <repository> <ref>' to recover\"\n}}",
        path
    )
}

fn derive_lifecycle_state(state: &crate::core::types::PersistedProvenanceState) -> &'static str {
    if state.detached {
        return "Detached";
    }
    if state.reconciliation.running {
        return "Reconciling";
    }
    match state.reconciliation.status {
        ReconciliationStatus::NeverRun => "Initialized",
        ReconciliationStatus::InProgress => "Reconciling",
        ReconciliationStatus::Success => "Converged",
        ReconciliationStatus::Failed => "Diverged",
    }
}

pub fn render_mount_dependency_summary(
    desired: &DesiredState,
    verification_results: &[VerificationResult],
) -> Option<String> {
    if desired.mount_declarations.is_empty() {
        return None;
    }
    let failures = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Failure)
        .count();
    Some(format!(
        "mounts refs={} dependencies={} verification_failures={}",
        desired.mount_declarations.len(),
        desired.mount_dependencies.len(),
        failures
    ))
}

pub fn render_deterministic_plan_summary(plan: &DeterministicReconciliationPlan) -> String {
    let view = build_plan_output(plan);
    let target = render_revision_with_requested_ref(
        &view.revision_context.target_revision,
        view.revision_context.requested_ref.as_deref(),
    );
    let baseline = view
        .revision_context
        .last_applied_revision
        .as_deref()
        .map(|previous| {
            render_previous_revision_with_requested_ref(
                previous,
                view.revision_context.last_applied_requested_ref.as_deref(),
            )
        })
        .unwrap_or_else(|| "none".to_string());
    format!(
        "deterministic_plan scope={} target=\"{}\" baseline=\"{}\" summary=\"{}\"",
        plan.scope_id,
        target,
        baseline,
        render_plan_count_summary(&view, false),
    )
}

pub fn render_plan_count_summary(view: &PlanOutputView, verbose: bool) -> String {
    let phrases = [
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Create),
            "create",
            "creates",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Update),
            "update",
            "updates",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Recover),
            "recover",
            "recovers",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Restart),
            "restart",
            "restarts",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Delete),
            "delete",
            "deletes",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Blocked),
            "blocked",
            "blocked",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::Skipped),
            "skipped",
            "skipped",
        ),
        summary_count_phrase(
            count_plan_entries(&view.entries, PlanEntryAction::NoOp),
            "unchanged",
            "unchanged",
        ),
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

pub fn render_apply_summary(
    view: &ApplyOutputView,
    convergence: Option<&DeterministicConvergenceRecord>,
) -> String {
    let counts = render_apply_count_summary(view);
    let outcome = convergence
        .map(|record| format!("Outcome: {}", humane_outcome_label(&record.status)))
        .unwrap_or_else(|| "Outcome: unknown".to_string());
    format!("{counts}\n{outcome}")
}

pub fn render_apply_count_summary(view: &ApplyOutputView) -> String {
    let mut counts = std::collections::BTreeMap::new();
    for event in &view.events {
        match event.state {
            crate::core::types::ExecutionState::Created => {
                *counts.entry("create").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Updated => {
                *counts.entry("update").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Deleted => {
                *counts.entry("delete").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Recovered => {
                *counts.entry("recover").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Restarted => {
                *counts.entry("restart").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Failed => {
                *counts.entry("failed").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Blocked => {
                *counts.entry("blocked").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Unchanged => {
                *counts.entry("unchanged").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Skipped => {
                *counts.entry("skipped").or_insert(0usize) += 1
            }
            crate::core::types::ExecutionState::Pending
            | crate::core::types::ExecutionState::Running => {}
        }
    }
    let order = [
        "create",
        "update",
        "recover",
        "restart",
        "delete",
        "failed",
        "blocked",
        "skipped",
        "unchanged",
    ];
    let parts = order
        .into_iter()
        .filter_map(|label| {
            counts
                .get(label)
                .copied()
                .filter(|count| *count > 0)
                .map(|count| match label {
                    "create" => summary_count_phrase(count, "create", "creates"),
                    "update" => summary_count_phrase(count, "update", "updates"),
                    "recover" => summary_count_phrase(count, "recover", "recovers"),
                    "restart" => summary_count_phrase(count, "restart", "restarts"),
                    "delete" => summary_count_phrase(count, "delete", "deletes"),
                    "failed" => summary_count_phrase(count, "failed", "failed"),
                    "blocked" => summary_count_phrase(count, "blocked", "blocked"),
                    "skipped" => summary_count_phrase(count, "skipped", "skipped"),
                    "unchanged" => summary_count_phrase(count, "unchanged", "unchanged"),
                    _ => format!("{count} {label}"),
                })
        })
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "0 changes".to_string()
    } else {
        parts.join(" • ")
    }
}

fn summary_count_phrase(count: usize, singular: &str, plural: &str) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

fn count_plan_entries(entries: &[PlanEntry], action: PlanEntryAction) -> usize {
    entries
        .iter()
        .filter(|entry| entry.action == action)
        .count()
}

pub fn render_revision_with_requested_ref(target: &str, requested_ref: Option<&str>) -> String {
    let primary = short_revision(target);
    match meaningful_requested_ref(target, requested_ref) {
        Some(requested_ref) => format!("{primary} ({requested_ref})"),
        None => primary.to_string(),
    }
}

pub fn render_previous_revision_with_requested_ref(
    previous: &str,
    requested_ref: Option<&str>,
) -> String {
    let primary = short_revision(previous);
    match meaningful_requested_ref(previous, requested_ref) {
        Some(requested_ref) => format!("{primary} ({requested_ref})"),
        None => primary.to_string(),
    }
}

fn meaningful_requested_ref<'a>(target: &str, requested_ref: Option<&'a str>) -> Option<&'a str> {
    let requested_ref = requested_ref?.trim();
    if requested_ref.is_empty() {
        return None;
    }
    if requested_ref == target {
        return None;
    }
    if target.starts_with(requested_ref) || requested_ref.starts_with(target) {
        return None;
    }
    Some(requested_ref)
}

fn short_revision(revision: &str) -> &str {
    &revision[..revision.len().min(8)]
}

fn humane_outcome_label(status: &crate::core::types::ConvergenceStatus) -> &'static str {
    match status {
        crate::core::types::ConvergenceStatus::Success => "converged",
        crate::core::types::ConvergenceStatus::Partial => "partially applied",
        crate::core::types::ConvergenceStatus::Blocked => "blocked",
        crate::core::types::ConvergenceStatus::RepeatedFailure
        | crate::core::types::ConvergenceStatus::Oscillation => "non-converging",
        crate::core::types::ConvergenceStatus::Failed => "convergence failed",
    }
}

pub fn render_rollback_summary(state: &DeterministicPersistedState) -> String {
    let target = state
        .latest_rollback_target
        .as_ref()
        .map(|candidate| candidate.target_revision_id.as_str())
        .unwrap_or("none");
    let eligibility = state
        .latest_rollback_target
        .as_ref()
        .map(|candidate| rollback_eligibility_label(&candidate.eligibility).to_string())
        .unwrap_or_else(|| "none".to_string());
    let convergence = state
        .latest_convergence
        .as_ref()
        .map(|record| convergence_status_label(&record.status).to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "rollback target={} eligibility={} convergence={}",
        target, eligibility, convergence
    )
}

pub fn render_convergence_summary(state: &DeterministicPersistedState) -> String {
    let convergence = state.latest_convergence.as_ref();
    format!(
        "convergence scope={} status={} attempts={} affected_objects={}",
        state.current_scope,
        convergence
            .map(|record| convergence_status_label(&record.status).to_string())
            .unwrap_or_else(|| "none".to_string()),
        convergence.map(|record| record.attempt_count).unwrap_or(0),
        convergence
            .map(|record| record.affected_objects.join(","))
            .unwrap_or_default()
    )
}

fn append_deterministic_status(base: String, provenance_path: &Path) -> String {
    let deterministic_path = provenance_path
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME);
    match read_deterministic_state(&deterministic_path) {
        Ok(Some(state)) => format!(
            "{base}\n{}\n{}",
            render_convergence_summary(&state),
            render_rollback_summary(&state)
        ),
        _ => base,
    }
}

fn rollback_eligibility_label(
    eligibility: &crate::core::types::RollbackEligibility,
) -> &'static str {
    match eligibility {
        crate::core::types::RollbackEligibility::Eligible => "eligible",
        crate::core::types::RollbackEligibility::MissingSnapshot => "missing_snapshot",
        crate::core::types::RollbackEligibility::IncompatibleScope => "incompatible_scope",
        crate::core::types::RollbackEligibility::Expired => "expired",
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
