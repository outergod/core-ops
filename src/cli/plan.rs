use crate::cli::report::{
    format_deterministic_plan_json, format_deterministic_plan_report,
    format_deterministic_plan_report_with_options_and_state, ApplyRunDisplayState,
};
use crate::core::audit::{build_audit_event, build_audit_record, summarize_evaluation, AuditEvent};
use crate::core::errors::CoreError;
use crate::core::evaluate::build_desired_snapshot_from_state;
use crate::core::reconcile::{
    normalize_verification_results_for_desired, reconcile_deterministic_plan_with_runtime,
    reconcile_plan, ReconcileDependencies,
};
use crate::core::types::AuditRecord;
use crate::core::verify::verify_state;
use crate::io::observed::build_observed_snapshot;
use crate::io::repo::HOST_OVERRIDE_ENV;
use crate::io::state::{
    latest_retained_snapshot_for_scope, read_deterministic_state, read_persisted_state,
    resolve_state_file, DETERMINISTIC_STATE_FILE_NAME,
};
use std::path::Path;

pub struct PlanOutput {
    pub summary: String,
    pub machine: String,
    pub audit_record: AuditRecord,
    pub audit_event: AuditEvent,
}

pub struct DeterministicPlanOutput {
    pub summary: String,
    pub machine: String,
}

pub fn plan(deps: &ReconcileDependencies<'_>, verbose: bool) -> Result<PlanOutput, CoreError> {
    let detached_header = detached_header_from_state()?;
    let result = reconcile_plan(deps)?;
    let observed = (deps.read_observed)(&result.desired)?;
    let scope_id = scope_id_for_observed(&observed);
    let desired_snapshot = build_desired_snapshot_from_state(&result.desired, &scope_id);
    let observed_snapshot = build_observed_snapshot(&observed, Some(&result.desired), &scope_id);
    let last_applied_revision = last_applied_revision_from_state()?;
    let last_applied_snapshot = last_applied_snapshot_for_scope(&scope_id)?;
    let verification_results = normalize_verification_results_for_desired(
        &result.desired,
        verify_state(&result.desired, &observed),
    );
    let run_display_state =
        classify_plan_run_display_state(last_applied_revision.as_deref(), &observed_snapshot);
    let mut deterministic = reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        last_applied_snapshot
            .as_ref()
            .map(|snapshot| &snapshot.snapshot),
        &observed_snapshot,
        &verification_results,
    )?;
    if deterministic.plan.baseline_revision_id.is_none() {
        if let Some(revision) = last_applied_revision {
            deterministic.plan.baseline_revision_id = Some(revision);
        }
    }
    deterministic.plan.requested_repository = result.desired.requested_repository.clone();
    deterministic.plan.requested_ref = result.desired.requested_ref.clone();
    deterministic.plan.last_applied_requested_repository = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_repository.clone());
    deterministic.plan.last_applied_requested_ref = last_applied_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.requested_ref.clone());
    let diffs = result.diffs;
    let mut audit = build_audit_record(&result.run.run_id, diffs.clone(), &result.plan, Vec::new());
    audit
        .operator_messages
        .push(summarize_evaluation(&result.desired));
    if !result.desired.mount_declarations.is_empty() {
        audit.operator_messages.push(format!(
            "mounts: native-artifacts={}, dependencies={}",
            result.desired.mount_declarations.len(),
            result.desired.mount_dependencies.len()
        ));
    }
    let event = build_audit_event(&result.run, Some(&result.plan), &[], None);

    let summary = format_deterministic_plan_report_with_options_and_state(
        &deterministic.plan,
        verbose,
        run_display_state,
    );
    let summary = if let Some(header) = detached_header {
        format!("{header}\n{summary}")
    } else {
        summary
    };

    Ok(PlanOutput {
        summary,
        machine: format_deterministic_plan_json(&deterministic.plan),
        audit_record: audit,
        audit_event: event,
    })
}

pub fn render_deterministic_plan(
    plan: &crate::core::types::DeterministicReconciliationPlan,
) -> DeterministicPlanOutput {
    // The machine-readable plan view is authoritative; the summary is a projection of it.
    DeterministicPlanOutput {
        summary: format_deterministic_plan_report(plan),
        machine: format_deterministic_plan_json(plan),
    }
}

fn scope_id_for_observed(observed: &crate::core::types::ObservedState) -> String {
    observed
        .host_info
        .as_ref()
        .map(|host| format!("host:{}", host.hostname))
        .or_else(|| {
            std::env::var(HOST_OVERRIDE_ENV)
                .ok()
                .filter(|host| !host.is_empty())
                .map(|host| format!("host:{host}"))
        })
        .or_else(default_host_scope_id)
        .unwrap_or_else(|| "scope:default".to_string())
}

fn default_host_scope_id() -> Option<String> {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if result != 0 {
        return None;
    }
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let hostname = String::from_utf8_lossy(&buf[..len]).trim().to_string();
    (!hostname.is_empty()).then(|| format!("host:{hostname}"))
}

fn detached_header_from_state() -> Result<Option<String>, CoreError> {
    use crate::core::errors::StateError;
    let state_path = resolve_state_file(None);
    let state = match read_persisted_state(&state_path) {
        Ok(state) => state,
        Err(StateError::Corrupt(path)) => {
            return Err(CoreError::new(
                crate::core::types::FailureClass::Plan,
                format!(
                    "state file at {} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover",
                    path
                ),
            ));
        }
        Err(err) => {
            return Err(CoreError::new(
                crate::core::types::FailureClass::Plan,
                format!("failed to read persisted state {}: {}", state_path.display(), err),
            ));
        }
    };
    if let Some(state) = state {
        if state.detached {
            let revision = state
                .reconciliation
                .last_applied_revision
                .as_deref()
                .unwrap_or("unknown");
            let requested_ref = &state.desired_state.requested_ref;
            return Ok(Some(format!(
                "[DETACHED] plan computed against detached revision {revision}; \
                 this represents what re-attaching to {requested_ref} would apply"
            )));
        }
    }
    Ok(None)
}

fn last_applied_revision_from_state() -> Result<Option<String>, CoreError> {
    let state_path = resolve_state_file(None);
    read_persisted_state(&state_path)
        .map_err(|err| {
            CoreError::new(
                crate::core::types::FailureClass::Plan,
                format!(
                    "failed to read persisted state {}: {}",
                    state_path.display(),
                    err
                ),
            )
        })
        .map(|state| state.and_then(|state| state.reconciliation.last_applied_revision))
}

fn last_applied_snapshot_for_scope(
    scope_id: &str,
) -> Result<Option<crate::core::types::RetainedAppliedSnapshot>, CoreError> {
    let state_path = resolve_state_file(None);
    let deterministic_state_path = deterministic_state_path(&state_path);
    let state = read_deterministic_state(&deterministic_state_path).map_err(|err| {
        CoreError::new(
            crate::core::types::FailureClass::Plan,
            format!(
                "failed to read deterministic state {}: {}",
                deterministic_state_path.display(),
                err
            ),
        )
    })?;
    Ok(state.and_then(|state| latest_retained_snapshot_for_scope(&state, scope_id).cloned()))
}

fn deterministic_state_path(status_path: &Path) -> std::path::PathBuf {
    status_path
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME)
}

fn classify_plan_run_display_state(
    last_applied_revision: Option<&str>,
    observed_snapshot: &crate::core::types::NormalizedSnapshot,
) -> ApplyRunDisplayState {
    if last_applied_revision.is_some() {
        ApplyRunDisplayState::Managed
    } else if observed_snapshot.objects.is_empty() {
        ApplyRunDisplayState::FirstRun
    } else {
        ApplyRunDisplayState::Recovery
    }
}
