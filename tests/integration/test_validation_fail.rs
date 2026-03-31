use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::core::types::{Boundaries, DesiredState, FailureClass, Invariant, ObservedState};

#[test]
fn validation_failure_is_reported_as_validation_class() {
    let desired = DesiredState {
        repository_ref: "fixture".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: Vec::new(),
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::DeterministicPlan],
        boundaries: Boundaries { scopes: Vec::new() },
    };

    let observed = ObservedState {
        observed_revision_id: None,
        units: Vec::new(),
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: None,
    };

    let deps = ReconcileDependencies {
        load_desired: &|| Ok(desired.clone()),
        read_observed: &|_desired| Ok(observed.clone()),
        apply_plan: &|_, _| Ok(()),
    };

    let err = match reconcile_plan(&deps) {
        Ok(_) => panic!("expected validation failure"),
        Err(err) => err,
    };
    assert_eq!(err.class, FailureClass::Validation);
}
