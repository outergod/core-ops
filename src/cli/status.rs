use std::path::{Path, PathBuf};

use crate::cli::report::build_plan_output;
use crate::core::types::{
    DesiredState, DeterministicPersistedState, DeterministicReconciliationPlan,
    VerificationResult, VerificationStatus,
};
use crate::io::state::{
    parse_persisted_state_text, read_deterministic_state, read_persisted_state, resolve_state_file,
    DETERMINISTIC_STATE_FILE_NAME,
};

pub fn render_status_from_path(path: &Path) -> String {
    let base = match read_persisted_state(path) {
        Ok(Some(state)) => render_present_state(&state),
        Ok(None) | Err(_) => absent_status(),
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
        None => absent_status(),
    }
}

fn render_present_state(state: &crate::core::types::PersistedProvenanceState) -> String {
    match serde_json::to_string_pretty(state) {
        Ok(pretty) => format!("provenance\n{}", pretty),
        Err(_) => absent_status(),
    }
}

fn absent_status() -> String {
    "provenance\n{\n  \"status\": \"absent\"\n}".to_string()
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
    format!(
        "deterministic_plan scope={} target_revision={} changed={} unchanged={} blocked={} skipped={}",
        plan.scope_id,
        view.revision_context.target_revision,
        view.summary.changed_count,
        view.summary.unchanged_count,
        view.summary.blocked_count,
        view.summary.skipped_count
    )
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
