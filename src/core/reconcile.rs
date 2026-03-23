use crate::core::diff::diff_workloads;
use crate::core::errors::CoreError;
use crate::core::planner::plan;
use crate::core::types::{
    DesiredState, DiffItem, FailureClass, PersistedProvenanceState, ReconcileMode, ReconcileRun,
    ReconciliationPlan, ReconciliationProvenance, ReconciliationStatus, RevisionDivergence,
    RunStatus, VerificationResult, VerificationStatus,
};
use crate::core::verify::verify_state;

pub struct ReconcileDependencies<'a> {
    pub load_desired: &'a dyn Fn() -> Result<crate::core::types::DesiredState, CoreError>,
    pub read_observed: &'a dyn Fn(
        &crate::core::types::DesiredState,
    ) -> Result<crate::core::types::ObservedState, CoreError>,
    pub apply_plan:
        &'a dyn Fn(&crate::core::types::ReconciliationPlan, &crate::core::types::DesiredState)
            -> Result<(), CoreError>,
}

pub struct PlanResult {
    pub run: ReconcileRun,
    pub plan: ReconciliationPlan,
    pub diffs: Vec<DiffItem>,
    pub desired: DesiredState,
}

pub struct ApplyResult {
    pub run: ReconcileRun,
    pub verification_results: Vec<VerificationResult>,
    pub desired: DesiredState,
}

pub fn next_reconciliation_generation(previous: Option<&PersistedProvenanceState>) -> u64 {
    previous
        .map(|state| state.reconciliation.generation + 1)
        .unwrap_or(1)
}

pub fn never_run_provenance() -> ReconciliationProvenance {
    ReconciliationProvenance {
        generation: 0,
        status: ReconciliationStatus::NeverRun,
        running: false,
        last_attempted_revision: None,
        last_applied_revision: None,
        last_started_at: None,
        last_finished_at: None,
        attempted_observed_divergence: None,
    }
}

pub fn build_reconciliation_provenance(
    previous: Option<&PersistedProvenanceState>,
    generation: u64,
    observed_revision: Option<&str>,
    attempted_revision: Option<&str>,
    status: ReconciliationStatus,
    started_at: Option<String>,
    finished_at: Option<String>,
) -> ReconciliationProvenance {
    if status == ReconciliationStatus::NeverRun {
        return never_run_provenance();
    }

    let attempted_revision = attempted_revision
        .or(observed_revision)
        .map(ToString::to_string);
    let attempted_observed_divergence = match (observed_revision, attempted_revision.as_deref()) {
        (Some(observed), Some(attempted)) if observed != attempted => Some(RevisionDivergence {
            observed_revision: observed.to_string(),
            attempted_revision: attempted.to_string(),
        }),
        _ => None,
    };
    let previous_applied = previous.and_then(|state| {
        state
            .reconciliation
            .last_applied_revision
            .as_ref()
            .map(ToString::to_string)
    });
    let last_applied_revision = match status {
        ReconciliationStatus::Success => attempted_revision.clone(),
        ReconciliationStatus::InProgress | ReconciliationStatus::Failed => previous_applied,
        ReconciliationStatus::NeverRun => None,
    };

    ReconciliationProvenance {
        generation,
        status: status.clone(),
        running: status == ReconciliationStatus::InProgress,
        last_attempted_revision: attempted_revision,
        last_applied_revision,
        last_started_at: started_at,
        last_finished_at: match status {
            ReconciliationStatus::InProgress => None,
            _ => finished_at,
        },
        attempted_observed_divergence,
    }
}

pub fn reconcile_plan(deps: &ReconcileDependencies<'_>) -> Result<PlanResult, CoreError> {
    let desired = (deps.load_desired)()?;
    let observed = (deps.read_observed)(&desired)?;

    let plan = plan(&desired, &observed)?;
    let diffs = diff_workloads(&desired.workloads, &observed.workloads);

    let run = ReconcileRun {
        run_id: format!("run:{}", plan.plan_id),
        mode: ReconcileMode::Plan,
        status: RunStatus::Success,
        failure_class: None,
        summary: "planned".to_string(),
    };

    Ok(PlanResult {
        run,
        plan,
        diffs,
        desired,
    })
}

pub fn reconcile_apply(deps: &ReconcileDependencies<'_>) -> Result<ApplyResult, CoreError> {
    let desired = (deps.load_desired)()?;
    let observed = (deps.read_observed)(&desired)?;

    let plan = plan(&desired, &observed)?;

    if !plan.actions.is_empty() {
        (deps.apply_plan)(&plan, &desired)?;
    }

    let observed_after = (deps.read_observed)(&desired)?;
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
        desired,
    })
}
