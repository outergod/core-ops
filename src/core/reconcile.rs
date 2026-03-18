use crate::core::diff::diff_workloads;
use crate::core::errors::CoreError;
use crate::core::planner::plan;
use crate::core::types::{FailureClass, ReconcileMode, ReconcileRun, RunStatus};

pub struct ReconcileDependencies<'a> {
    pub load_desired: &'a dyn Fn() -> Result<crate::core::types::DesiredState, CoreError>,
    pub read_observed: &'a dyn Fn() -> Result<crate::core::types::ObservedState, CoreError>,
    pub apply_plan:
        &'a dyn Fn(&crate::core::types::ReconciliationPlan, &crate::core::types::DesiredState)
            -> Result<(), CoreError>,
}

pub fn reconcile_apply(deps: &ReconcileDependencies<'_>) -> Result<ReconcileRun, CoreError> {
    let desired = (deps.load_desired)()?;
    let observed = (deps.read_observed)()?;

    let plan = plan(&desired, &observed)?;

    if !plan.actions.is_empty() {
        (deps.apply_plan)(&plan, &desired)?;
    }

    let observed_after = (deps.read_observed)()?;
    let diffs = diff_workloads(&desired.workloads, &observed_after.workloads);

    let (status, failure_class, summary) = if diffs.is_empty() {
        (RunStatus::Success, None, "converged".to_string())
    } else {
        (
            RunStatus::Failure,
            Some(FailureClass::Verify),
            "verification failed".to_string(),
        )
    };

    Ok(ReconcileRun {
        run_id: format!("run:{}", plan.plan_id),
        mode: ReconcileMode::Apply,
        status,
        failure_class,
        summary,
    })
}
