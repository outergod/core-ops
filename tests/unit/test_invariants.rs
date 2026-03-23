use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, Invariant,
};
use core_ops::core::validation::validate_desired_state;

#[test]
fn idempotent_apply_invariant_is_allowed() {
    let desired = DesiredState {
        repository_ref: "fixture".to_string(),
        revision_id: "rev".to_string(),
        workloads: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![
            Invariant::BoundariesDeclared,
            Invariant::DeterministicPlan,
            Invariant::IdempotentApply,
        ],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };

    let result = validate_desired_state(&desired);
    assert!(result.is_ok());
}
