use std::fs;
use std::path::PathBuf;

use core_ops::core::types::{
    ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState,
    ReconciliationProvenance, ReconciliationStatus, TreeState, PERSISTED_PROVENANCE_SCHEMA_VERSION,
};
use core_ops::io::state::{read_persisted_state, write_persisted_state};

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
fn partial_snapshot_is_treated_as_absent() {
    let path = temp_file("state_partial.json");
    fs::write(&path, "{\n  \"schema_version\": 1,\n  \"controller\": {")
        .expect("write partial state");

    let loaded = read_persisted_state(&path).expect("read partial state");
    assert_eq!(loaded, None);
    let _ = fs::remove_file(path);
}

#[test]
fn unsupported_schema_snapshot_is_treated_as_absent() {
    let path = temp_file("state_unsupported.json");
    let mut state = fixture_state();
    state.schema_version = 99;
    fs::write(
        &path,
        serde_json::to_vec_pretty(&state).expect("serialize unsupported"),
    )
    .expect("write unsupported state");

    let loaded = read_persisted_state(&path).expect("read unsupported state");
    assert_eq!(loaded, None);
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

fn fixture_state() -> PersistedProvenanceState {
    PersistedProvenanceState {
        schema_version: PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: ControllerProvenance {
            version: Some("0.1.0".to_string()),
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
    }
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
