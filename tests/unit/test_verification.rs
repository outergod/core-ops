use core_ops::core::types::{
    DesiredState, ObservedState, QuadletType, UnitActiveState, VerificationStatus, Workload,
    EnabledState, RestartPolicy, Invariant, Boundaries, BoundaryScope, ObservedUnit,
};
use core_ops::core::verify::verify_state;

fn workload(name: &str, quadlet_type: QuadletType, unit: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type,
        quadlet_contents: "[Unit]".to_string(),
        systemd_unit_name: unit.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn desired_state(workloads: Vec<Workload>) -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads,
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn observed_state(units: Vec<ObservedUnit>) -> ObservedState {
    ObservedState {
        observed_revision_id: None,
        units,
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: None,
    }
}

#[test]
fn verify_container_requires_active_unit() {
    let desired = desired_state(vec![workload("alpha", QuadletType::Container, "alpha.container")]);
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "alpha.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Enabled,
    }]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, VerificationStatus::Failure);
}

#[test]
fn verify_volume_accepts_loaded_unit() {
    let desired = desired_state(vec![workload("gamma", QuadletType::Volume, "gamma.volume")]);
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "gamma.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Disabled,
    }]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, VerificationStatus::Success);
}
