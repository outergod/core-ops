use core_ops::core::types::{
    Boundaries, BoundaryScope, EnabledState, Invariant, QuadletType, RestartPolicy,
    Workload,
};

#[test]
fn boundaries_reports_scopes() {
    let boundaries = Boundaries {
        scopes: vec![BoundaryScope::QuadletSystemd],
    };

    assert!(boundaries.has_scope(BoundaryScope::QuadletSystemd));
}

#[test]
fn workload_key_is_name() {
    let workload = Workload {
        name: "alpha".to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: "[Container]".to_string(),
        systemd_unit_name: "alpha.container".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };

    assert_eq!(workload.key(), "alpha");
}

#[test]
fn invariants_can_be_listed_explicitly() {
    let invariants = vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan];

    assert!(invariants.contains(&Invariant::BoundariesDeclared));
    assert!(invariants.contains(&Invariant::DeterministicPlan));
}
