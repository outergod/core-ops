use std::cell::Cell;

use core_ops::core::reconcile::{reconcile_apply_with_retry, ReconcileDependencies};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, HostInfo, Invariant, ObservedState,
    ObservedUnit, QuadletType, RestartPolicy, UnitActiveState, Workload,
};

fn desired_state() -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev-1".to_string(),
        workloads: vec![Workload {
            name: "alpha".to_string(),
            quadlet_type: QuadletType::Container,
            quadlet_contents: "[Container]\nImage=alpine".to_string(),
            systemd_unit_name: "alpha.container".to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        }],
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

#[test]
fn repeated_failure_detection_stops_after_bounded_retry_budget() {
    let desired = desired_state();
    let observations = Cell::new(0usize);
    let deps = ReconcileDependencies {
        load_desired: &|| Ok(desired.clone()),
        read_observed: &|_| {
            observations.set(observations.get() + 1);
            Ok(ObservedState {
                observed_revision_id: Some("obs".to_string()),
                units: vec![ObservedUnit {
                    unit_name: "alpha.service".to_string(),
                    active_state: UnitActiveState::Inactive,
                    enabled_state: EnabledState::Enabled,
                }],
                workloads: Vec::new(),
                last_reconcile_id: None,
                host_info: Some(HostInfo {
                    hostname: "alpha".to_string(),
                    os_id: "fedora".to_string(),
                }),
            })
        },
        apply_plan: &|_, _| Ok(()),
    };

    let result = reconcile_apply_with_retry(&deps, 3).expect("retry reconcile");

    assert_eq!(result.run.status, core_ops::core::types::RunStatus::Failure);
    assert_eq!(
        result.convergence.as_ref().map(|record| &record.status),
        Some(&core_ops::core::types::ConvergenceStatus::RepeatedFailure)
    );
    assert_eq!(
        result.convergence.as_ref().map(|record| record.attempt_count),
        Some(3)
    );
    assert!(observations.get() >= 3);
}
