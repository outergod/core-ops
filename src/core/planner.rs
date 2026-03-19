use crate::core::boundaries::enforce_plan_boundaries;
use crate::core::diff::diff_workloads;
use crate::core::errors::{CoreError, ValidationError};
use crate::core::types::{
    DiffItem, DiffKind, FailureClass, PlanAction, PlanActionType, QuadletType,
    ReconciliationPlan, SafetyCheck, DesiredState, ObservedState,
};
use crate::core::validation::validate_desired_state;

pub fn plan(desired: &DesiredState, observed: &ObservedState) -> Result<ReconciliationPlan, CoreError> {
    validate_desired_state(desired).map_err(map_validation_error)?;

    let mut diffs = diff_workloads(&desired.workloads, &observed.workloads);
    order_diffs(&mut diffs);
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
            action(PlanActionType::StartUnit, name),
        ],
        DiffKind::Remove => vec![
            action(PlanActionType::StopUnit, name),
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

fn order_diffs(diffs: &mut [DiffItem]) {
    diffs.sort_by(|a, b| {
        let a_key = ordering_key(a);
        let b_key = ordering_key(b);
        a_key.cmp(&b_key)
    });
}

fn ordering_key(diff: &DiffItem) -> (u8, String) {
    let quadlet_type = diff
        .desired
        .as_ref()
        .or(diff.observed.as_ref())
        .map(|w| w.quadlet_type.clone());

    let order = match diff.kind {
        DiffKind::Remove => reverse_order_for_type(quadlet_type),
        _ => order_for_type(quadlet_type),
    };

    (order, diff.name.clone())
}

fn order_for_type(quadlet_type: Option<QuadletType>) -> u8 {
    match quadlet_type {
        Some(QuadletType::Volume) => 0,
        Some(QuadletType::Container) => 1,
        Some(QuadletType::Socket) => 2,
        Some(QuadletType::Pod) => 3,
        Some(QuadletType::Network) => 4,
        None => 5,
    }
}

fn reverse_order_for_type(quadlet_type: Option<QuadletType>) -> u8 {
    match quadlet_type {
        Some(QuadletType::Socket) => 0,
        Some(QuadletType::Container) => 1,
        Some(QuadletType::Volume) => 2,
        Some(QuadletType::Pod) => 3,
        Some(QuadletType::Network) => 4,
        None => 5,
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
