use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, PlanAction,
    PlanActionType, QuadletType, ReconciliationPlan, RestartPolicy, Workload,
};
use core_ops::core::validation::validate_desired_state;
use core_ops::core::boundaries::enforce_plan_boundaries;

fn base_desired() -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![Workload {
            name: "alpha".to_string(),
            quadlet_type: QuadletType::Container,
            quadlet_contents: "[Container]".to_string(),
            systemd_unit_name: "alpha.container".to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        }],
        managed_config_paths: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

#[test]
fn validates_ok_when_invariants_and_boundaries_present() {
    let desired = base_desired();

    assert!(validate_desired_state(&desired).is_ok());
}

#[test]
fn rejects_unsupported_plan_actions() {
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![
            PlanAction {
                action_type: PlanActionType::ReloadSystemd,
                target: "alpha".to_string(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            PlanAction {
                action_type: PlanActionType::WriteQuadlet,
                target: "alpha".to_string(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
        ],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let result = enforce_plan_boundaries(&plan);
    assert!(result.is_ok());
}

#[test]
fn rejects_unknown_plan_action_types() {
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![PlanAction {
            action_type: PlanActionType::Unknown("mystery".to_string()),
            target: "alpha".to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let result = enforce_plan_boundaries(&plan);
    assert!(result.is_err());
}

#[test]
fn fails_when_missing_invariant() {
    let mut desired = base_desired();
    desired.invariants = vec![Invariant::BoundariesDeclared];

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("DeterministicPlan"));
}

#[test]
fn fails_when_missing_boundary_scope() {
    let mut desired = base_desired();
    desired.boundaries.scopes.clear();

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("QuadletSystemd"));
}

#[test]
fn allows_same_name_for_distinct_unit_names() {
    let mut desired = base_desired();
    let mut extra = desired.workloads[0].clone();
    extra.quadlet_type = QuadletType::Socket;
    extra.systemd_unit_name = "alpha.socket".to_string();
    desired.workloads.push(extra);

    assert!(validate_desired_state(&desired).is_ok());
}

#[test]
fn fails_on_duplicate_unit_name() {
    let mut desired = base_desired();
    let mut extra = desired.workloads[0].clone();
    extra.name = "beta".to_string();
    desired.workloads.push(extra);

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("duplicate unit"));
}
