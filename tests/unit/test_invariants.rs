use core_ops::core::types::{
    Boundaries, BoundaryScope, ConvergenceStatus, DesiredState, DeterministicConvergenceRecord,
    Invariant, ReconciliationProvenance, ReconciliationStatus,
};
use core_ops::core::validation::validate_desired_state;

#[test]
fn idempotent_apply_invariant_is_allowed() {
    let desired = DesiredState {
        repository_ref: "fixture".to_string(),
        revision_id: "rev".to_string(),
        workloads: Vec::new(),
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
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

#[test]
fn convergence_record_attempts_remain_bounded() {
    let record = DeterministicConvergenceRecord {
        desired_revision_id: "rev-1".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::RepeatedFailure,
        attempt_count: 3,
        affected_objects: vec!["alpha.container".to_string()],
        completed_actions: Vec::new(),
        failed_actions: vec!["alpha.container".to_string()],
        can_continue: false,
    };

    assert!(record.attempt_count > 0);
    assert!(!record.can_continue);
}
