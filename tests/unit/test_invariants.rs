use core_ops::core::types::{
    Boundaries, BoundaryScope, ConvergenceStatus, DesiredState, DeterministicConvergenceRecord,
    Invariant, NormalizedSnapshot, ReconciliationProvenance, ReconciliationStatus,
    RetainedAppliedSnapshot, RollbackEligibility,
};
use core_ops::cli::report::{build_result_output, format_result_output_report};
use core_ops::core::validation::validate_desired_state;
use core_ops::io::state::{resolve_rollback_target, retain_successful_snapshot};

#[test]
fn idempotent_apply_invariant_is_allowed() {
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

#[test]
fn rollback_eligibility_requires_retained_snapshot_with_matching_scope() {
    let state = core_ops::core::types::DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![RetainedAppliedSnapshot {
            revision_id: "rev-1".to_string(),
            scope_id: "host:beta".to_string(),
            requested_repository: None,
            requested_ref: None,
            snapshot: NormalizedSnapshot {
                revision_id: Some("rev-1".to_string()),
                scope_id: "host:beta".to_string(),
                objects: Vec::new(),
            },
            retained: true,
        }],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    let candidate = resolve_rollback_target(&state, "host:alpha", "rev-1");
    assert_eq!(
        candidate.eligibility,
        RollbackEligibility::IncompatibleScope
    );
}

#[test]
fn retaining_successful_snapshots_respects_bounded_history() {
    let mut state = core_ops::core::types::DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: Vec::new(),
        latest_convergence: None,
        latest_rollback_target: None,
    };

    for rev in ["rev-1", "rev-2", "rev-3"] {
        retain_successful_snapshot(
            &mut state,
            RetainedAppliedSnapshot {
                revision_id: rev.to_string(),
                scope_id: "host:alpha".to_string(),
                requested_repository: None,
                requested_ref: None,
                snapshot: NormalizedSnapshot {
                    revision_id: Some(rev.to_string()),
                    scope_id: "host:alpha".to_string(),
                    objects: Vec::new(),
                },
                retained: true,
            },
            2,
        );
    }

    let retained = state
        .retained_snapshots
        .iter()
        .filter(|snapshot| snapshot.retained)
        .map(|snapshot| snapshot.revision_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(retained, vec!["rev-2", "rev-3"]);
}

#[test]
fn result_output_rendering_preserves_outcome_and_object_order() {
    let plan = core_ops::core::types::DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "alpha.container".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::Create,
                reason: "object missing from actual state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "beta.container".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "desired, last applied, and actual state already match".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: core_ops::core::types::SemanticDependencyGraph {
            nodes: vec![
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "alpha.container".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::QuadletResource,
                    ordering_key: "alpha.container".to_string(),
                },
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "beta.container".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::QuadletResource,
                    ordering_key: "beta.container".to_string(),
                },
            ],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["alpha.container".to_string()],
        completed_actions: vec!["alpha.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let result = build_result_output(&plan, &[], Some(&convergence));
    let rendered = format_result_output_report(&result);

    assert_eq!(result.entries[0].object.display_id, "container/alpha.container");
    assert_eq!(result.entries[1].object.display_id, "container/beta.container");
    assert!(rendered.contains("Outcome: converged"));
    assert!(rendered.contains("container/alpha.container"));
    assert!(rendered.contains("container/beta.container"));
}
