use std::path::Path;

use crate::core::errors::CoreError;
use crate::core::reconcile::{reconcile_apply, reconcile_plan, ReconcileDependencies};
use crate::core::types::{FailureClass, ReconcileRun, ReconciliationStatus, RunStatus};
use crate::cli::report::{append_provenance_report, format_plan_report};
use crate::io::apply::apply_plan;
use crate::io::observed::read_observed_state;
use crate::io::repo::load_desired_state;
use crate::io::state::{persist_finished_state, persist_in_progress_state, resolve_state_file};

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
            apply_plan(plan, &desired.workloads, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let result = reconcile_apply(&deps)?;
    Ok(result.run)
}

use crate::core::reconcile::ApplyResult;

pub fn apply_with_report(
    repo_source: &str,
    revision: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<(ApplyResult, String, crate::core::types::ReconciliationPlan), CoreError> {
    let repo_source = repo_source.to_string();
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo_source, revision).map_err(map_plan_error),
        read_observed: &|desired| {
            read_observed_state(quadlet_dir, Some(desired), None).map_err(map_plan_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let state_path = resolve_state_file(None);
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
    let result = reconcile_apply(&deps)?;
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
