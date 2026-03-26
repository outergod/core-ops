use crate::core::diff::{diff_contains_mount_workloads, diff_workloads};
use crate::core::errors::CoreError;
use crate::core::planner::{plan, plan_deterministic_reconciliation, plan_rollback_reconciliation};
use crate::core::retry::{build_retry_observation, RetryObservation};
use crate::core::types::{
    ConvergenceStatus, DesiredState, DeterministicConvergenceRecord,
    DeterministicReconciliationPlan, DiffItem, FailureClass, NormalizedSnapshot,
    PersistedProvenanceState, ReconcileMode, ReconcileRun, ReconciliationPlan,
    ReconciliationProvenance, ReconciliationStatus, RevisionDivergence, RollbackEligibility,
    RollbackTargetCandidate, RunStatus, VerificationResult, VerificationStatus,
};
use crate::core::verify::{evaluate_convergence, verify_state};
use crate::io::state::{
    latest_retained_snapshot_for_scope, resolve_rollback_target, retained_snapshot_for_target,
};

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
    pub convergence: Option<DeterministicConvergenceRecord>,
}

pub struct DeterministicPlanResult {
    pub plan: DeterministicReconciliationPlan,
    pub summary: String,
}

#[derive(Debug)]
pub struct RollbackResult {
    pub target: RollbackTargetCandidate,
    pub plan: DeterministicReconciliationPlan,
    pub convergence: Option<DeterministicConvergenceRecord>,
    pub summary: String,
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

    let summary = if diff_contains_mount_workloads(&diffs) {
        "planned mount-backed changes".to_string()
    } else {
        "planned".to_string()
    };
    let run = ReconcileRun {
        run_id: format!("run:{}", plan.plan_id),
        mode: ReconcileMode::Plan,
        status: RunStatus::Success,
        failure_class: None,
        summary,
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
    let has_degraded_mount = verification_results.iter().any(|result| {
        result
            .details
            .as_deref()
            .map(|details| details.starts_with("degraded:"))
            .unwrap_or(false)
    });
    let has_blocked_mount = verification_results.iter().any(|result| {
        result
            .details
            .as_deref()
            .map(|details| details.starts_with("blocked:"))
            .unwrap_or(false)
    });

    let (status, failure_class, summary) = if diffs.is_empty() && !has_failures {
        (
            RunStatus::Success,
            None,
            if !desired.mount_declarations.is_empty() {
                "converged mount-backed services".to_string()
            } else {
                "converged".to_string()
            },
        )
    } else {
        (
            RunStatus::Failure,
            Some(FailureClass::Verify),
            if !desired.mount_declarations.is_empty() {
                if has_degraded_mount {
                    "mount degraded".to_string()
                } else if has_blocked_mount {
                    "mount blocked".to_string()
                } else {
                    "mount verification failed".to_string()
                }
            } else {
                "verification failed".to_string()
            },
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
        convergence: None,
    })
}

pub fn reconcile_deterministic_plan(
    desired: &NormalizedSnapshot,
    last_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
) -> Result<DeterministicPlanResult, CoreError> {
    let plan = plan_deterministic_reconciliation(desired, last_applied, actual)?;
    let summary = format!(
        "deterministic plan scope={} desired_revision={}",
        plan.scope_id,
        plan.desired_revision_id.as_deref().unwrap_or("none")
    );
    Ok(DeterministicPlanResult { plan, summary })
}

pub fn reconcile_rollback(
    state: &crate::core::types::DeterministicPersistedState,
    scope_id: &str,
    target_revision_id: &str,
    actual: &NormalizedSnapshot,
) -> Result<RollbackResult, CoreError> {
    let target = resolve_rollback_target(state, scope_id, target_revision_id);
    if target.eligibility != RollbackEligibility::Eligible {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "rollback target {} is {:?}: {}",
                target.target_revision_id, target.eligibility, target.reason
            ),
        ));
    }

    let target_snapshot = retained_snapshot_for_target(state, scope_id, target_revision_id)
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                format!("rollback target {} snapshot is unavailable", target_revision_id),
            )
        })?;
    let current_snapshot = latest_retained_snapshot_for_scope(state, scope_id)
        .map(|snapshot| &snapshot.snapshot);
    let plan = plan_rollback_reconciliation(
        &target_snapshot.snapshot,
        current_snapshot,
        actual,
        target_revision_id,
    )?;
    let convergence = Some(DeterministicConvergenceRecord {
        desired_revision_id: target_revision_id.to_string(),
        scope_id: scope_id.to_string(),
        status: if plan
            .actions
            .iter()
            .any(|action| matches!(action.classification, crate::core::types::DeterministicActionClass::Blocked))
        {
            ConvergenceStatus::Blocked
        } else {
            ConvergenceStatus::Partial
        },
        attempt_count: 1,
        affected_objects: plan.actions.iter().map(|action| action.object_id.clone()).collect(),
        completed_actions: Vec::new(),
        failed_actions: plan
            .actions
            .iter()
            .filter(|action| matches!(action.classification, crate::core::types::DeterministicActionClass::Blocked))
            .map(|action| action.object_id.clone())
            .collect(),
        can_continue: true,
    });
    let summary = format!(
        "rollback target={} baseline={} actions={}",
        target.target_revision_id,
        plan.baseline_revision_id.as_deref().unwrap_or("none"),
        plan.actions.len()
    );

    Ok(RollbackResult {
        target,
        plan,
        convergence,
        summary,
    })
}

pub fn reconcile_apply_with_retry(
    deps: &ReconcileDependencies<'_>,
    retry_budget: u32,
) -> Result<ApplyResult, CoreError> {
    let mut history = Vec::<RetryObservation>::new();
    let mut last_result = None;

    for attempt in 1..=retry_budget.max(1) {
        let mut result = reconcile_apply(deps)?;
        let observed = (deps.read_observed)(&result.desired)?;
        let verification_results = verify_state(&result.desired, &observed);
        let observation = build_retry_observation(attempt, &verification_results);
        history.push(observation);
        let convergence = evaluate_convergence(&result.desired, &observed, &history, retry_budget.max(1));

        result.verification_results = verification_results;
        result.run.summary = convergence_summary(&convergence);
        result.run.status = if convergence.status == ConvergenceStatus::Success {
            RunStatus::Success
        } else {
            RunStatus::Failure
        };
        result.run.failure_class = if result.run.status == RunStatus::Failure {
            Some(FailureClass::Verify)
        } else {
            None
        };
        result.convergence = Some(convergence.clone());

        let stop = matches!(
            convergence.status,
            ConvergenceStatus::Success
                | ConvergenceStatus::Blocked
                | ConvergenceStatus::RepeatedFailure
                | ConvergenceStatus::Oscillation
        ) || attempt == retry_budget.max(1);
        last_result = Some(result);
        if stop {
            break;
        }
    }

    last_result.ok_or_else(|| CoreError::new(FailureClass::Apply, "retry orchestration produced no result"))
}

fn convergence_summary(convergence: &DeterministicConvergenceRecord) -> String {
    match convergence.status {
        ConvergenceStatus::Success => "converged".to_string(),
        ConvergenceStatus::Partial => "partial convergence".to_string(),
        ConvergenceStatus::Blocked => "blocked prerequisites".to_string(),
        ConvergenceStatus::RepeatedFailure => "repeated failure detected".to_string(),
        ConvergenceStatus::Oscillation => "oscillation detected".to_string(),
        ConvergenceStatus::Failed => "verification failed".to_string(),
    }
}
