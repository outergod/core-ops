use crate::core::boundaries::enforce_plan_boundaries;
use crate::core::diff::diff_workloads;
use crate::core::errors::{CoreError, ValidationError};
use crate::core::types::{
    DiffItem, DiffKind, FailureClass, PlanAction, PlanActionType, QuadletType,
    ReconciliationPlan, SafetyCheck, DesiredState, ObservedState,
};
use crate::core::validation::validate_desired_state;
use std::collections::HashSet;
use std::path::Path;

pub fn plan(desired: &DesiredState, observed: &ObservedState) -> Result<ReconciliationPlan, CoreError> {
    validate_desired_state(desired).map_err(map_validation_error)?;

    let mut diffs = diff_workloads(&desired.workloads, &observed.workloads);
    order_diffs(&mut diffs);
    let mut actions = Vec::new();

    let socket_stems = desired_socket_stems(&desired.workloads);
    let container_stems = desired_container_stems(&desired.workloads);
    for diff in &diffs {
        let quadlet_type = diff
            .desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|workload| workload.quadlet_type.clone());
        let mut diff_actions =
            actions_for_diff(
                diff.kind.clone(),
                &diff.name,
                quadlet_type,
                &socket_stems,
                &container_stems,
            );
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

fn actions_for_diff(
    kind: DiffKind,
    name: &str,
    quadlet_type: Option<QuadletType>,
    socket_stems: &HashSet<String>,
    container_stems: &HashSet<String>,
) -> Vec<PlanAction> {
    let manage_unit = match quadlet_type {
        Some(QuadletType::SocketDropIn) => false,
        Some(QuadletType::ConfigFile) => false,
        Some(QuadletType::Volume) => false,
        Some(QuadletType::Container) => {
            let stem = stem_for_unit_name(name);
            match stem {
                Some(stem) => !socket_stems.contains(stem),
                None => true,
            }
        }
        _ => true,
    };
    let reload_systemd = !matches!(quadlet_type, Some(QuadletType::ConfigFile));
    match kind {
        DiffKind::Add => {
            let mut actions = vec![
                action(PlanActionType::WriteQuadlet, name),
            ];
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            if manage_unit {
                actions.push(action(PlanActionType::StartUnit, name));
            }
            if should_start_service_for_socket(quadlet_type.as_ref(), name, container_stems) {
                actions.push(action(
                    PlanActionType::StartUnit,
                    &format!("{}.service", stem_for_unit_name(name).unwrap_or(name)),
                ));
            }
            actions
        }
        DiffKind::Remove => {
            let mut actions = Vec::new();
            if manage_unit {
                actions.push(action(PlanActionType::StopUnit, name));
            }
            actions.push(action(PlanActionType::RemoveQuadlet, name));
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            actions
        }
        DiffKind::Change => {
            let mut actions = vec![
                action(PlanActionType::WriteQuadlet, name),
            ];
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            if manage_unit {
                actions.push(action(PlanActionType::StartUnit, name));
            }
            if should_start_service_for_socket(quadlet_type.as_ref(), name, container_stems) {
                actions.push(action(
                    PlanActionType::StartUnit,
                    &format!("{}.service", stem_for_unit_name(name).unwrap_or(name)),
                ));
            }
            actions
        }
    }
}

fn desired_socket_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Socket)
        .filter_map(|workload| stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string()))
        .collect()
}

fn desired_container_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Container)
        .filter_map(|workload| stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string()))
        .collect()
}

fn stem_for_unit_name(name: &str) -> Option<&str> {
    Path::new(name).file_stem().and_then(|stem| stem.to_str())
}

fn should_start_service_for_socket(
    quadlet_type: Option<&QuadletType>,
    name: &str,
    container_stems: &HashSet<String>,
) -> bool {
    if !matches!(quadlet_type, Some(QuadletType::Socket)) {
        return false;
    }
    match stem_for_unit_name(name) {
        Some(stem) => container_stems.contains(stem),
        None => false,
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
        Some(QuadletType::ConfigFile) => 0,
        Some(QuadletType::Volume) => 1,
        Some(QuadletType::Network) => 2,
        Some(QuadletType::Container) => 3,
        Some(QuadletType::Socket) => 4,
        Some(QuadletType::SocketDropIn) => 5,
        Some(QuadletType::Pod) => 6,
        None => 7,
    }
}

fn reverse_order_for_type(quadlet_type: Option<QuadletType>) -> u8 {
    match quadlet_type {
        Some(QuadletType::SocketDropIn) => 0,
        Some(QuadletType::Socket) => 1,
        Some(QuadletType::Container) => 2,
        Some(QuadletType::Volume) => 3,
        Some(QuadletType::Network) => 4,
        Some(QuadletType::ConfigFile) => 5,
        Some(QuadletType::Pod) => 6,
        None => 7,
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
