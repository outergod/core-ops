use crate::core::boundaries::enforce_plan_boundaries;
use crate::core::diff::diff_workloads;
use crate::core::errors::{CoreError, ValidationError};
use crate::core::types::{
    DiffKind, FailureClass, PlanAction, PlanActionType, ReconciliationPlan,
    SafetyCheck, DesiredState, ObservedState,
};
use crate::core::validation::validate_desired_state;

pub fn plan(desired: &DesiredState, observed: &ObservedState) -> Result<ReconciliationPlan, CoreError> {
    validate_desired_state(desired).map_err(map_validation_error)?;

    let diffs = diff_workloads(&desired.workloads, &observed.workloads);
    let mut actions = Vec::new();

    for diff in &diffs {
        let mut diff_actions = actions_for_diff(diff.kind.clone(), &diff.name);
        actions.append(&mut diff_actions);
    }

    let plan_id = format!(
        "plan:{}:{}",
        desired.revision_id,
        observed
            .observed_revision_id
            .clone()
            .unwrap_or_else(|| "none".to_string())
    );

    let plan = ReconciliationPlan {
        plan_id,
        desired_revision_id: desired.revision_id.clone(),
        observed_revision_id: observed.observed_revision_id.clone(),
        actions,
        safety_checks: vec![
            SafetyCheck::BoundariesDeclared,
            SafetyCheck::SupportedQuadletTypes,
            SafetyCheck::DeterministicPlan,
        ],
        expected_outcomes: vec!["observed state converges to desired state".to_string()],
    };

    enforce_plan_boundaries(&plan)?;
    Ok(plan)
}

fn actions_for_diff(kind: DiffKind, name: &str) -> Vec<PlanAction> {
    match kind {
        DiffKind::Add => vec![
            action(PlanActionType::WriteQuadlet, name),
            action(PlanActionType::ReloadSystemd, name),
            action(PlanActionType::EnableUnit, name),
            action(PlanActionType::StartUnit, name),
        ],
        DiffKind::Remove => vec![
            action(PlanActionType::StopUnit, name),
            action(PlanActionType::DisableUnit, name),
            action(PlanActionType::RemoveQuadlet, name),
            action(PlanActionType::ReloadSystemd, name),
        ],
        DiffKind::Change => vec![
            action(PlanActionType::WriteQuadlet, name),
            action(PlanActionType::ReloadSystemd, name),
            action(PlanActionType::StartUnit, name),
        ],
    }
}

fn action(action_type: PlanActionType, name: &str) -> PlanAction {
    PlanAction {
        action_type,
        target: name.to_string(),
        preconditions: Vec::new(),
        postconditions: Vec::new(),
    }
}

fn map_validation_error(err: ValidationError) -> CoreError {
    CoreError::new(FailureClass::Validation, err.message)
}
