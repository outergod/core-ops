use std::path::Path;

use crate::core::errors::CoreError;
use crate::core::reconcile::{reconcile_apply, reconcile_plan, ReconcileDependencies};
use crate::core::types::{FailureClass, ReconcileRun};
use crate::io::apply::apply_plan;
use crate::io::observed::read_observed_state;
use crate::io::repo::load_desired_state;
use crate::cli::report::format_plan_report;

pub fn apply(
    repo_source: &str,
    revision: &str,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ReconcileRun, CoreError> {
    let repo_source = repo_source.to_string();
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo_source, revision).map_err(map_plan_error),
        read_observed: &|| read_observed_state(quadlet_dir, None).map_err(map_plan_error),
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
        read_observed: &|| read_observed_state(quadlet_dir, None).map_err(map_plan_error),
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, quadlet_dir, reload_systemd)
                .map(|_| ())
                .map_err(map_apply_error)
        },
    };

    let plan_result = reconcile_plan(&deps)?;
    let report = format_plan_report(&plan_result.plan, &plan_result.diffs);
    let result = reconcile_apply(&deps)?;
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
