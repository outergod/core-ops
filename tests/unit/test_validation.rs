use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, QuadletType,
    RestartPolicy, Workload,
};
use core_ops::core::validation::validate_desired_state;

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
fn fails_on_duplicate_workload_name() {
    let mut desired = base_desired();
    let mut extra = desired.workloads[0].clone();
    extra.systemd_unit_name = "beta.container".to_string();
    desired.workloads.push(extra);

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("duplicate workload"));
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
