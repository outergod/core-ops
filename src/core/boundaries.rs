use crate::core::errors::CoreError;
use crate::core::types::{FailureClass, PlanAction, PlanActionType, ReconciliationPlan};

pub fn enforce_plan_boundaries(plan: &ReconciliationPlan) -> Result<(), CoreError> {
    for action in &plan.actions {
        validate_action(action)?;
    }
    Ok(())
}

fn validate_action(action: &PlanAction) -> Result<(), CoreError> {
    if is_supported_action(&action.action_type) {
        return Ok(());
    }

    Err(CoreError::new(
        FailureClass::Validation,
        format!("unsupported mutation action: {:?}", action.action_type),
    ))
}

fn is_supported_action(action_type: &PlanActionType) -> bool {
    matches!(
        action_type,
        PlanActionType::WriteQuadlet
            | PlanActionType::RemoveQuadlet
            | PlanActionType::EnableUnit
            | PlanActionType::DisableUnit
            | PlanActionType::ReloadSystemd
            | PlanActionType::StartUnit
            | PlanActionType::StopUnit
    )
}
