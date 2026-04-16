use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use core_ops::core::types::{
    ControllerProvenance, ConvergenceStatus, DesiredStateProvenance,
    DeterministicConvergenceRecord, DeterministicPersistedState, ManagedObjectKind,
    NormalizedManagedObject, NormalizedSnapshot, PersistedProvenanceState,
    ReconciliationProvenance, ReconciliationStatus, RetainedAppliedSnapshot, RollbackEligibility,
    RollbackTargetCandidate, TreeState, PERSISTED_PROVENANCE_SCHEMA_VERSION,
};
use core_ops::core::errors::StateError;
use core_ops::io::state::{
    default_deterministic_state_path, default_state_file_path, persist_finished_state,
    persist_in_progress_state, read_deterministic_state, read_persisted_state, resolve_state_file,
    write_deterministic_state, write_persisted_state, STATE_FILE_ENV,
};

#[test]
fn valid_snapshot_round_trips_through_state_io() {
    let path = temp_file("state_round_trip.json");
    let state = fixture_state();

    write_persisted_state(&path, &state).expect("write state");
    let loaded = read_persisted_state(&path).expect("read state");

    assert_eq!(loaded, Some(state));
    let _ = fs::remove_file(path);
}

#[test]
fn partial_snapshot_is_treated_as_corrupt() {
    let path = temp_file("state_partial.json");
    fs::write(&path, "{\n  \"schema_version\": 1,\n  \"controller\": {")
        .expect("write partial state");

    let result = read_persisted_state(&path);
    assert!(
        matches!(result, Err(StateError::Corrupt(_))),
        "expected Corrupt, got {:?}",
        result
    );
    let _ = fs::remove_file(path);
}

#[test]
fn unsupported_schema_snapshot_is_treated_as_corrupt() {
    let path = temp_file("state_unsupported.json");
    let mut state = fixture_state();
    state.schema_version = 99;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("serialize unsupported"),
    )
    .expect("write unsupported state");

    let result = read_persisted_state(&path);
    assert!(
        matches!(result, Err(StateError::Corrupt(_))),
        "expected Corrupt, got {:?}",
        result
    );
    let _ = fs::remove_file(path);
}

#[test]
fn supported_schema_snapshot_remains_readable_after_round_trip() {
    let path = temp_file("state_supported_schema.json");
    let state = fixture_state();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("serialize supported state"),
    )
    .expect("write supported state");

    let loaded = read_persisted_state(&path).expect("read supported state");
    assert_eq!(loaded, Some(state));
    let _ = fs::remove_file(path);
}

#[test]
fn running_snapshot_requires_in_progress_status() {
    let mut state = fixture_state();
    state.reconciliation.running = true;

    assert!(!state.reconciliation.is_valid());

    state.reconciliation.status = ReconciliationStatus::InProgress;
    state.reconciliation.last_finished_at = None;
    assert!(state.reconciliation.is_valid());
}

#[test]
fn reconciliation_generation_and_transitions_remain_monotonic() {
    let path = temp_file("state_transitions.json");
    let state = fixture_state();
    write_persisted_state(&path, &state).expect("write initial state");

    let attempt = persist_in_progress_state(
        &path,
        "file:///var/lib/core-ops/repo",
        "main",
        "feedface",
        None,
    )
    .expect("persist in progress");
    assert_eq!(attempt.generation, 2);

    let in_progress = read_persisted_state(&path)
        .expect("read in-progress")
        .expect("snapshot exists");
    assert_eq!(
        in_progress.reconciliation.status,
        ReconciliationStatus::InProgress
    );
    assert_eq!(in_progress.reconciliation.generation, 2);
    assert_eq!(
        in_progress.reconciliation.last_applied_revision.as_deref(),
        Some("deadbeef")
    );

    persist_finished_state(
        &path,
        "file:///var/lib/core-ops/repo",
        "main",
        "feedface",
        None,
        &attempt,
        ReconciliationStatus::Failed,
    )
    .expect("persist failed state");

    let failed = read_persisted_state(&path)
        .expect("read failed state")
        .expect("failed snapshot exists");
    assert_eq!(failed.reconciliation.status, ReconciliationStatus::Failed);
    assert_eq!(failed.reconciliation.generation, 2);
    assert_eq!(
        failed.reconciliation.last_attempted_revision.as_deref(),
        Some("feedface")
    );
    assert_eq!(
        failed.reconciliation.last_applied_revision.as_deref(),
        Some("deadbeef")
    );

    let attempt = persist_in_progress_state(
        &path,
        "file:///var/lib/core-ops/repo",
        "main",
        "cafebabe",
        None,
    )
    .expect("persist second in progress");
    assert_eq!(attempt.generation, 3);

    persist_finished_state(
        &path,
        "file:///var/lib/core-ops/repo",
        "main",
        "cafebabe",
        None,
        &attempt,
        ReconciliationStatus::Success,
    )
    .expect("persist success state");

    let success = read_persisted_state(&path)
        .expect("read success state")
        .expect("success snapshot exists");
    assert_eq!(success.reconciliation.status, ReconciliationStatus::Success);
    assert_eq!(success.reconciliation.generation, 3);
    assert_eq!(
        success.reconciliation.last_applied_revision.as_deref(),
        Some("cafebabe")
    );

    let _ = fs::remove_file(path);
}

#[test]
fn persisted_state_does_not_require_history_or_journal_fields() {
    let state = fixture_state();
    let value = serde_json::to_value(&state).expect("serialize state");
    let object = value.as_object().expect("state object");

    assert!(object.get("history").is_none());
    assert!(object.get("journal").is_none());
    assert!(object.get("events").is_none());
}

#[test]
fn state_file_resolution_defaults_to_canonical_path() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::remove_var(STATE_FILE_ENV);
    assert_eq!(resolve_state_file(None), default_state_file_path());
}

#[test]
fn state_file_resolution_uses_env_override_before_default() {
    let _guard = env_lock().lock().expect("env lock");
    let path = temp_file("state_env_override.json");
    std::env::set_var(STATE_FILE_ENV, &path);

    assert_eq!(resolve_state_file(None), path);

    std::env::remove_var(STATE_FILE_ENV);
}

#[test]
fn deterministic_state_round_trips_with_retained_snapshots() {
    let path = temp_file("deterministic_state.json");
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![RetainedAppliedSnapshot {
            revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            requested_repository: None,
            requested_ref: None,
            snapshot: NormalizedSnapshot {
                revision_id: Some("rev-1".to_string()),
                scope_id: "host:alpha".to_string(),
                objects: vec![NormalizedManagedObject {
                    object_id: "alpha.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    material_fields: Default::default(),
                    dependency_refs: Vec::new(),
                }],
            },
            retained: true,
        }],
        latest_convergence: Some(DeterministicConvergenceRecord {
            desired_revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            status: ConvergenceStatus::Success,
            attempt_count: 1,
            affected_objects: vec!["alpha.container".to_string()],
            completed_actions: vec!["alpha.container".to_string()],
            failed_actions: Vec::new(),
            can_continue: true,
        }),
        latest_rollback_target: Some(RollbackTargetCandidate {
            target_revision_id: "rev-0".to_string(),
            scope_id: "host:alpha".to_string(),
            eligibility: RollbackEligibility::Eligible,
            reason: "retained".to_string(),
        }),
    };

    write_deterministic_state(&path, &state).expect("write deterministic state");
    let loaded = read_deterministic_state(&path).expect("read deterministic state");
    assert_eq!(loaded, Some(state));
    let _ = fs::remove_file(path);
}

#[test]
fn deterministic_state_default_path_uses_runtime_dir() {
    let path = default_deterministic_state_path();
    assert!(path.ends_with("deterministic-state.json"));
}

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn fixture_state() -> PersistedProvenanceState {
    PersistedProvenanceState {
        schema_version: PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: ControllerProvenance {
            version: Some("0.4.0".to_string()),
            revision: Some("abc1234".to_string()),
            build_time: Some("2026-03-23T10:00:00Z".to_string()),
            tree_state: TreeState::Clean,
        },
        desired_state: DesiredStateProvenance {
            repository: "file:///var/lib/core-ops/repo".to_string(),
            requested_ref: "main".to_string(),
            last_observed_revision: Some("deadbeef".to_string()),
            last_observed_at: Some("2026-03-23T10:05:00Z".to_string()),
        },
        reconciliation: ReconciliationProvenance {
            generation: 1,
            status: ReconciliationStatus::Success,
            running: false,
            last_attempted_revision: Some("deadbeef".to_string()),
            last_applied_revision: Some("deadbeef".to_string()),
            last_started_at: Some("2026-03-23T10:06:00Z".to_string()),
            last_finished_at: Some("2026-03-23T10:06:09Z".to_string()),
            attempted_observed_divergence: None,
        },
        detached: false,
    }
}

// T029: read_persisted_state with invalid JSON returns Corrupt with file path; absent returns Ok(None)
#[test]
fn corrupt_state_error_contains_file_path() {
    let path = temp_file("corrupt_path_check.json");
    fs::write(&path, "not valid json").expect("write invalid json");

    let result = read_persisted_state(&path);
    match result {
        Err(StateError::Corrupt(msg)) => {
            assert!(
                msg.contains(path.to_str().unwrap()),
                "corrupt error should contain file path, got: {msg}"
            );
        }
        other => panic!("expected Corrupt, got {:?}", other),
    }
    let _ = fs::remove_file(&path);

    // absent file returns Ok(None)
    let absent_result = read_persisted_state(&path);
    assert!(
        matches!(absent_result, Ok(None)),
        "absent file should return Ok(None), got {:?}",
        absent_result
    );
}

fn temp_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    path.push(format!("core_ops_{stamp}_{name}"));
    path
}
