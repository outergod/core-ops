use std::path::PathBuf;

use core_ops::cli::args::InitArgs;
use core_ops::cli::init::run_init;
use core_ops::core::types::{ReconciliationStatus, PERSISTED_PROVENANCE_SCHEMA_VERSION};
use core_ops::io::state::{
    persist_never_run_state, read_persisted_state, write_persisted_state,
};

fn temp_state_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    path.push(format!("core_ops_init_{stamp}_{name}"));
    path
}

fn init_args(
    path: &std::path::Path,
    repository: &str,
    requested_ref: &str,
    force: bool,
) -> InitArgs {
    InitArgs {
        repository: repository.to_string(),
        requested_ref: requested_ref.to_string(),
        force,
        state_file: Some(path.to_path_buf()),
    }
}

// (a) success on absent state writes NeverRun state with correct fields
#[test]
fn init_on_absent_state_writes_never_run() {
    let path = temp_state_file("absent.json");

    run_init(&init_args(&path, "file:///repo", "main", false)).expect("init should succeed");

    let state = read_persisted_state(&path)
        .expect("read")
        .expect("state exists");
    assert_eq!(
        state.reconciliation.status,
        ReconciliationStatus::NeverRun
    );
    assert_eq!(state.desired_state.repository, "file:///repo");
    assert_eq!(state.desired_state.requested_ref, "main");
    assert!(!state.detached);
    assert_eq!(state.schema_version, PERSISTED_PROVENANCE_SCHEMA_VERSION);

    let _ = std::fs::remove_file(path);
}

// (b) init without --force on valid existing state returns 'already initialized' error
#[test]
fn init_without_force_on_existing_state_returns_already_initialized() {
    let path = temp_state_file("existing.json");
    persist_never_run_state(&path, "file:///repo", "main").expect("setup");

    let err = run_init(&init_args(&path, "file:///repo", "main", false))
        .expect_err("should fail");
    assert!(
        err.message.contains("already initialized"),
        "unexpected error: {}",
        err.message
    );

    let _ = std::fs::remove_file(path);
}

// (c) init on corrupt state without --force returns 'corrupt state' error containing file path
#[test]
fn init_without_force_on_corrupt_state_returns_corrupt_error() {
    let path = temp_state_file("corrupt.json");
    std::fs::write(&path, "{\"schema_version\": 1, \"broken\":").expect("write corrupt");

    let err = run_init(&init_args(&path, "file:///repo", "main", false))
        .expect_err("should fail");
    assert!(
        err.message.contains("corrupt"),
        "unexpected error: {}",
        err.message
    );
    assert!(
        err.message.contains(path.to_str().unwrap()),
        "error should contain path: {}",
        err.message
    );

    let _ = std::fs::remove_file(path);
}

// (f) ref validation accepts a branch name
#[test]
fn init_accepts_branch_name_as_ref() {
    let path = temp_state_file("branch.json");
    run_init(&init_args(&path, "file:///repo", "feature/my-branch", false))
        .expect("branch name should be accepted");
    let _ = std::fs::remove_file(path);
}

// (g) ref validation accepts a tag name
#[test]
fn init_accepts_tag_as_ref() {
    let path = temp_state_file("tag.json");
    run_init(&init_args(&path, "file:///repo", "v1.2.3", false))
        .expect("tag name should be accepted");
    let _ = std::fs::remove_file(path);
}

// (h) init --force with same repo/ref preserves reconciliation history and clears detached flag
#[test]
fn init_force_same_config_preserves_history_and_clears_detached() {
    let path = temp_state_file("preserve.json");
    // Write an initial state with some reconciliation history
    persist_never_run_state(&path, "file:///repo", "main").expect("setup never-run");
    // Simulate a completed reconciliation
    core_ops::io::state::persist_success_state(
        &path,
        "file:///repo",
        "main",
        "deadbeef",
    )
    .expect("persist success");

    // Mark as detached
    let mut state = read_persisted_state(&path)
        .expect("read")
        .expect("exists");
    state.detached = true;
    write_persisted_state(&path, &state).expect("write detached");

    // re-init with --force, same repo/ref
    run_init(&init_args(&path, "file:///repo", "main", true)).expect("force reinit");

    let after = read_persisted_state(&path)
        .expect("read")
        .expect("exists");
    // History should be preserved (generation unchanged, last_applied_revision still set)
    assert_eq!(
        after.reconciliation.last_applied_revision.as_deref(),
        Some("deadbeef"),
        "history should be preserved"
    );
    assert!(!after.detached, "detached should be cleared");

    let _ = std::fs::remove_file(path);
}

// (i) init --force with different repo/ref resets to NeverRun and clears retained snapshots
#[test]
fn init_force_different_config_resets_to_never_run() {
    let path = temp_state_file("reset.json");
    persist_never_run_state(&path, "file:///old-repo", "main").expect("setup");
    core_ops::io::state::persist_success_state(
        &path,
        "file:///old-repo",
        "main",
        "deadbeef",
    )
    .expect("persist success");

    // re-init with --force, different repo
    run_init(&init_args(&path, "file:///new-repo", "main", true)).expect("force reinit");

    let after = read_persisted_state(&path)
        .expect("read")
        .expect("exists");
    assert_eq!(
        after.reconciliation.status,
        ReconciliationStatus::NeverRun,
        "status should reset to NeverRun"
    );
    assert_eq!(after.desired_state.repository, "file:///new-repo");
    assert!(
        after.reconciliation.last_applied_revision.is_none(),
        "history should be cleared"
    );

    let _ = std::fs::remove_file(path);
}

// (j) init on a host where the state file parent directory does not exist creates the directory
#[test]
fn init_creates_parent_directory_when_missing() {
    let base = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    let nested = base.join(format!("core_ops_init_{stamp}_nested_dir")).join("subdir");
    let path = nested.join("status.json");

    assert!(!nested.exists(), "test precondition: directory should not exist");

    run_init(&init_args(&path, "file:///repo", "main", false))
        .expect("init should create parent directory");

    assert!(path.exists(), "state file should exist after init");

    let _ = std::fs::remove_dir_all(nested);
}

// (k) deserialization of a state file JSON without 'detached' field produces detached == false
#[test]
fn state_without_detached_field_deserializes_as_not_detached() {
    let path = temp_state_file("no_detached.json");
    // Write a valid state JSON without the detached field
    let json = r#"{
        "schema_version": 1,
        "controller": {
            "version": null,
            "revision": null,
            "build_time": null,
            "tree_state": "unknown"
        },
        "desired_state": {
            "repository": "file:///repo",
            "requested_ref": "main",
            "last_observed_revision": null,
            "last_observed_at": null
        },
        "reconciliation": {
            "generation": 0,
            "status": "never_run",
            "running": false,
            "last_attempted_revision": null,
            "last_applied_revision": null,
            "last_started_at": null,
            "last_finished_at": null,
            "attempted_observed_divergence": null
        }
    }"#;
    std::fs::write(&path, json).expect("write state without detached field");

    let state = read_persisted_state(&path)
        .expect("read")
        .expect("state exists");
    assert!(!state.detached, "missing detached field should default to false");

    let _ = std::fs::remove_file(path);
}
