use std::path::Path;

use crate::core::evaluate::build_desired_snapshot_from_state;
use crate::core::errors::CoreError;
use crate::core::reconcile::{
    reconcile_apply, reconcile_apply_with_retry, reconcile_plan, reconcile_rollback,
    ReconcileDependencies,
};
use crate::core::types::{
    DeterministicPersistedState, FailureClass, ReconcileRun, ReconciliationStatus,
    RetainedAppliedSnapshot, RollbackTargetCandidate, RunStatus,
};
use crate::cli::report::{
    append_provenance_report, format_convergence_report_json, format_plan_report,
    format_rollback_report,
};
use crate::io::apply::apply_plan_with_desired;
use crate::io::observed::{build_observed_snapshot, read_observed_state};
use crate::io::repo::load_desired_state;
use crate::io::state::{
    persist_finished_state, persist_in_progress_state, read_deterministic_state,
    read_deterministic_state as load_deterministic_state, record_convergence_outcome,
    record_rollback_outcome, retain_successful_snapshot, write_deterministic_state,
    DETERMINISTIC_STATE_FILE_NAME,
};

pub fn apply(
    repo_source: &str,
    revision: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ReconcileRun, CoreError> {
    let repo_source = repo_source.to_string();
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo_source, revision).map_err(map_plan_error),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let result = reconcile_apply(&deps)?;
    Ok(result.run)
}

use crate::core::reconcile::ApplyResult;

const DEFAULT_RETRY_BUDGET: u32 = 3;
const DEFAULT_ROLLBACK_HISTORY: usize = 10;

pub fn apply_with_report(
    repo_source: &str,
    revision: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
    state_path: Option<std::path::PathBuf>,
) -> Result<(ApplyResult, String, crate::core::types::ReconciliationPlan), CoreError> {
    let repo_source = repo_source.to_string();
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo_source, revision).map_err(map_plan_error),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let plan_result = reconcile_plan(&deps)?;
    let mut report = format_plan_report(&plan_result.plan, &plan_result.diffs);
    let attempt = match state_path.as_ref() {
        Some(path) => Some(
            persist_in_progress_state(
                path,
                &repo_source,
                revision,
                &plan_result.desired.revision_id,
                None,
            )
            .map_err(map_apply_error)?,
        ),
        None => None,
    };
    let result = reconcile_apply_with_retry(&deps, DEFAULT_RETRY_BUDGET)?;
    if result
        .desired
        .mount_declarations
        .iter()
        .any(|mount| mount.automount)
    {
        report.push_str("\nautomount apply behavior active");
    }
    report.push('\n');
    report.push_str(&format_convergence_report_json(
        &result.run,
        &result.verification_results,
        result.convergence.as_ref(),
    ));

    if let Some(status_path) = state_path.as_ref() {
        let deterministic_state_path = deterministic_state_path(status_path);
        let mut deterministic_state = load_or_init_deterministic_state(&deterministic_state_path)
            .map_err(map_apply_error)?;
        if let Some(convergence) = result.convergence.clone() {
            let desired_snapshot =
                build_desired_snapshot_from_state(&result.desired, &convergence.scope_id);
            record_convergence_outcome(&mut deterministic_state, convergence);
            if result.run.status == RunStatus::Success {
                retain_successful_snapshot(
                    &mut deterministic_state,
                    RetainedAppliedSnapshot {
                        revision_id: result.desired.revision_id.clone(),
                        scope_id: desired_snapshot.scope_id.clone(),
                        snapshot: desired_snapshot,
                        retained: true,
                    },
                    DEFAULT_ROLLBACK_HISTORY,
                );
            }
            write_deterministic_state(&deterministic_state_path, &deterministic_state)
                .map_err(map_apply_error)?;
        }
    }
    if let (Some(path), Some(attempt)) = (state_path.as_ref(), attempt.as_ref()) {
        let status = match result.run.status {
            RunStatus::Success => ReconciliationStatus::Success,
            RunStatus::Failure => ReconciliationStatus::Failed,
        };
        persist_finished_state(
            path,
            &repo_source,
            revision,
            &result.desired.revision_id,
            None,
            attempt,
            status,
        )
        .map_err(map_apply_error)?;
        let contents = std::fs::read_to_string(path).map_err(map_apply_error)?;
        report = append_provenance_report(&report, Some(&contents));
    }
    Ok((result, report, plan_result.plan))
}

pub fn rollback_with_report(
    deterministic_state_path: &Path,
    target_revision_id: &str,
    actual: &crate::core::types::NormalizedSnapshot,
) -> Result<String, CoreError> {
    let state = read_deterministic_state(deterministic_state_path)
        .map_err(map_apply_error)?
        .ok_or_else(|| CoreError::new(FailureClass::Apply, "deterministic rollback state is absent"))?;
    let result = reconcile_rollback(&state, &actual.scope_id, target_revision_id, actual)?;
    Ok(format_rollback_report(&result.target, &result.plan))
}

pub fn execute_rollback_with_report(
    repo_source: &str,
    target_revision_id: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
    state_path: Option<std::path::PathBuf>,
    plan_only: bool,
) -> Result<(ApplyResult, String, crate::core::types::ReconciliationPlan), CoreError> {
    let state_path = state_path.ok_or_else(|| {
        CoreError::new(
            FailureClass::Apply,
            "rollback requires a persisted state path and deterministic history",
        )
    })?;
    let deterministic_state_path = deterministic_state_path(&state_path);
    let desired = load_desired_state(repo_source, target_revision_id).map_err(map_plan_error)?;
    let observed =
        read_observed_state(quadlet_dir, Some(&desired), None).map_err(map_plan_error)?;
    let actual = build_observed_snapshot(&observed, &scope_id_for_observed(&observed));
    let preview = rollback_with_report(&deterministic_state_path, target_revision_id, &actual)?;

    if plan_only {
        let run = ReconcileRun {
            run_id: format!("run:rollback-plan:{target_revision_id}"),
            mode: crate::core::types::ReconcileMode::Plan,
            status: RunStatus::Success,
            failure_class: None,
            summary: format!("rollback plan ready for {}", target_revision_id),
        };
        return Ok((
            ApplyResult {
                run,
                verification_results: Vec::new(),
                desired,
                convergence: None,
            },
            preview,
            reconcile_plan(&ReconcileDependencies {
                load_desired: &|| load_desired_state(repo_source, target_revision_id).map_err(map_plan_error),
                read_observed: &|desired| {
                    read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
                },
                apply_plan: &|plan, desired| {
                    apply_plan_with_desired(plan, desired, quadlet_dir, reload_systemd)
                        .map(|_| ())
                        .map_err(map_apply_error)
                },
            })?
            .plan,
        ));
    }

    let (result, apply_report, plan) = apply_with_report(
        repo_source,
        target_revision_id,
        quadlet_dir,
        reload_systemd,
        Some(state_path.clone()),
    )?;

    let mut deterministic_state = load_or_init_deterministic_state(&deterministic_state_path)
        .map_err(map_apply_error)?;
    if let Some(convergence) = result.convergence.clone() {
        record_rollback_outcome(
            &mut deterministic_state,
            RollbackTargetCandidate {
                target_revision_id: target_revision_id.to_string(),
                scope_id: convergence.scope_id.clone(),
                eligibility: crate::core::types::RollbackEligibility::Eligible,
                reason: "retained successful snapshot is rollback-eligible".to_string(),
            },
            convergence,
        );
        write_deterministic_state(&deterministic_state_path, &deterministic_state)
            .map_err(map_apply_error)?;
    }

    Ok((result, format!("{preview}\n{apply_report}"), plan))
}

fn deterministic_state_path(status_path: &Path) -> std::path::PathBuf {
    status_path
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME)
}

fn load_or_init_deterministic_state(
    path: &Path,
) -> Result<DeterministicPersistedState, crate::core::errors::StateError> {
    load_deterministic_state(path)?.map_or_else(
        || {
            Ok(DeterministicPersistedState {
                schema_version: 1,
                current_scope: "scope:default".to_string(),
                retained_snapshots: Vec::new(),
                latest_convergence: None,
                latest_rollback_target: None,
            })
        },
        Ok,
    )
}

fn scope_id_for_observed(observed: &crate::core::types::ObservedState) -> String {
    observed
        .host_info
        .as_ref()
        .map(|host| format!("host:{}", host.hostname))
        .unwrap_or_else(|| "scope:default".to_string())
}

fn map_plan_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: FailureClass::Plan,
        message: err.to_string(),
    }
}

fn map_apply_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: FailureClass::Apply,
        message: err.to_string(),
    }
}
