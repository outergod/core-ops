use crate::core::diff::diff_workloads;
use crate::core::errors::CoreError;
use crate::core::planner::plan;
use crate::core::types::{
    DiffItem, FailureClass, ReconcileMode, ReconcileRun, ReconciliationPlan, RunStatus,
    VerificationResult, VerificationStatus,
};
use crate::core::verify::verify_state;

pub struct ReconcileDependencies<'a> {
    pub load_desired: &'a dyn Fn() -> Result<crate::core::types::DesiredState, CoreError>,
    pub read_observed: &'a dyn Fn() -> Result<crate::core::types::ObservedState, CoreError>,
    pub apply_plan:
        &'a dyn Fn(&crate::core::types::ReconciliationPlan, &crate::core::types::DesiredState)
            -> Result<(), CoreError>,
}

pub struct PlanResult {
    pub run: ReconcileRun,
    pub plan: ReconciliationPlan,
    pub diffs: Vec<DiffItem>,
}

pub struct ApplyResult {
    pub run: ReconcileRun,
    pub verification_results: Vec<VerificationResult>,
}

pub fn reconcile_plan(deps: &ReconcileDependencies<'_>) -> Result<PlanResult, CoreError> {
    let desired = (deps.load_desired)()?;
    let observed = (deps.read_observed)()?;

    let plan = plan(&desired, &observed)?;
    let diffs = diff_workloads(&desired.workloads, &observed.workloads);

    let run = ReconcileRun {
        run_id: format!("run:{}", plan.plan_id),
        mode: ReconcileMode::Plan,
        status: RunStatus::Success,
        failure_class: None,
        summary: "planned".to_string(),
    };

    Ok(PlanResult { run, plan, diffs })
}

pub fn reconcile_apply(deps: &ReconcileDependencies<'_>) -> Result<ApplyResult, CoreError> {
    let desired = (deps.load_desired)()?;
    let observed = (deps.read_observed)()?;

    let plan = plan(&desired, &observed)?;

    if !plan.actions.is_empty() {
        (deps.apply_plan)(&plan, &desired)?;
    }

    let observed_after = (deps.read_observed)()?;
    let diffs = diff_workloads(&desired.workloads, &observed_after.workloads);
    let verification_results = verify_state(&desired, &observed_after);
    let has_failures = verification_results
        .iter()
        .any(|result| result.status == VerificationStatus::Failure);

    let (status, failure_class, summary) = if diffs.is_empty() && !has_failures {
        (RunStatus::Success, None, "converged".to_string())
    } else {
        (
            RunStatus::Failure,
            Some(FailureClass::Verify),
            "verification failed".to_string(),
        )
    };

    let run = ReconcileRun {
        run_id: format!("run:{}", plan.plan_id),
        mode: ReconcileMode::Apply,
        status,
        failure_class,
        summary,
    };

    Ok(ApplyResult {
        run,
        verification_results,
    })
}
