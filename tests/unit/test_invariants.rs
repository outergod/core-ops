use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, Invariant, ReconciliationProvenance,
    ReconciliationStatus,
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

#[test]
fn in_progress_reconciliation_cannot_have_finished_timestamp() {
    let reconciliation = ReconciliationProvenance {
        generation: 1,
        status: ReconciliationStatus::InProgress,
        running: true,
        last_attempted_revision: Some("rev-1".to_string()),
        last_applied_revision: Some("rev-0".to_string()),
        last_started_at: Some("2026-03-23T10:06:00Z".to_string()),
        last_finished_at: Some("2026-03-23T10:06:09Z".to_string()),
        attempted_observed_divergence: None,
    };

    assert!(!reconciliation.is_valid());
}
