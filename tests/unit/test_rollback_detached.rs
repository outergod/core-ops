use std::path::PathBuf;

use core_ops::cli::agent::{run_agent, AgentConfig, AgentExitReason};
use core_ops::core::reconcile::never_run_provenance;
use core_ops::core::types::{
    ControllerProvenance, DesiredStateProvenance, NormalizedSnapshot, PersistedProvenanceState,
    ReconciliationStatus, RetainedAppliedSnapshot, RollbackEligibility, TreeState,
    PERSISTED_PROVENANCE_SCHEMA_VERSION,
};
use core_ops::core::types::DeterministicPersistedState;
use core_ops::io::state::{
    read_persisted_state, resolve_rollback_target, write_persisted_state,
};

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    path.push(format!("core_ops_rollback_{stamp}_{name}"));
    path
}

fn detached_state_fixture(revision: &str) -> PersistedProvenanceState {
    let mut state = PersistedProvenanceState {
        schema_version: PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: ControllerProvenance {
            version: None,
            revision: None,
            build_time: None,
            tree_state: TreeState::Unknown,
        },
        desired_state: DesiredStateProvenance {
            repository: "file:///repo".to_string(),
            requested_ref: "main".to_string(),
            last_observed_revision: None,
            last_observed_at: None,
            layout_version: None,
        },
        reconciliation: never_run_provenance(),
        detached: false,
    };
    // Simulate a success then mark detached
    state.reconciliation.status = ReconciliationStatus::Success;
    state.reconciliation.last_applied_revision = Some(revision.to_string());
    state.reconciliation.last_attempted_revision = Some(revision.to_string());
    state.reconciliation.last_started_at = Some("2026-01-01T00:00:00Z".to_string());
    state.reconciliation.last_finished_at = Some("2026-01-01T00:01:00Z".to_string());
    state.detached = true;
    state
}

// T019(b): FR-014 rollback eligibility does not require Git reachability
// - A retained snapshot for a revision is eligible even if the revision isn't reachable in the current repo
#[test]
fn rollback_eligibility_does_not_depend_on_git_reachability() {
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:testhost".to_string(),
        retained_snapshots: vec![RetainedAppliedSnapshot {
            revision_id: "deadbeef12345678".to_string(),
            scope_id: "host:testhost".to_string(),
            requested_repository: None,
            requested_ref: None,
            snapshot: NormalizedSnapshot {
                revision_id: Some("deadbeef12345678".to_string()),
                scope_id: "host:testhost".to_string(),
                objects: vec![],
            },
            retained: true,
        }],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    // The revision exists in retained_snapshots — eligibility must be Eligible
    // without any Git operation being performed
    let candidate =
        resolve_rollback_target(&state, "host:testhost", "deadbeef12345678");
    assert_eq!(
        candidate.eligibility,
        RollbackEligibility::Eligible,
        "retained snapshot should be eligible without Git reachability check"
    );
}

#[test]
fn rollback_eligibility_rejects_missing_snapshot() {
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:testhost".to_string(),
        retained_snapshots: vec![],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    let candidate = resolve_rollback_target(&state, "host:testhost", "nonexistent");
    assert_eq!(candidate.eligibility, RollbackEligibility::MissingSnapshot);
}

// T020: run_agent with detached state returns Detached exit reason with revision
#[test]
fn agent_detached_state_returns_detached_exit_with_revision() {
    let path = temp_path("agent_detached.json");
    let state = detached_state_fixture("cafebabe");
    write_persisted_state(&path, &state).expect("write detached state");

    let config = AgentConfig {
        quadlet_dir: PathBuf::from("/etc/containers/systemd"),
        audit_dir: None,
        state_file: Some(path.clone()),
        reload_systemd: false,
        lock_path: Some(temp_path("agent_lock")),
    };

    let result = run_agent(&config).expect("run_agent should succeed");
    match result {
        AgentExitReason::Detached { revision } => {
            assert_eq!(revision, "cafebabe", "detached revision should match applied revision");
        }
        other => panic!("expected Detached, got {:?}", other),
    }

    let _ = std::fs::remove_file(path);
}

// T020: verify agent does NOT call apply when detached (state file unchanged after agent run)
#[test]
fn agent_detached_state_does_not_modify_reconciliation() {
    let path = temp_path("agent_no_reconcile.json");
    let state = detached_state_fixture("deadbeef");
    write_persisted_state(&path, &state).expect("write detached state");

    let config = AgentConfig {
        quadlet_dir: PathBuf::from("/etc/containers/systemd"),
        audit_dir: None,
        state_file: Some(path.clone()),
        reload_systemd: false,
        lock_path: Some(temp_path("agent_lock_no_reconcile")),
    };

    let _ = run_agent(&config).expect("agent should exit cleanly");

    let after = read_persisted_state(&path)
        .expect("read")
        .expect("state exists");
    assert!(after.detached, "state should still be detached");
    assert_eq!(
        after.reconciliation.last_applied_revision.as_deref(),
        Some("deadbeef"),
        "reconciliation should be unchanged"
    );

    let _ = std::fs::remove_file(path);
}
