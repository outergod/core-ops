use std::path::PathBuf;

use core_ops::cli::agent::{run_agent, AgentConfig, AgentExitReason};

fn temp_state_file(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("unix epoch")
        .as_nanos();
    path.push(format!("core_ops_agent_{stamp}_{name}"));
    path
}

// T010: run_agent with no state file returns Uninitialized (clean exit), not an error
#[test]
fn agent_with_no_state_file_returns_uninitialized() {
    let path = temp_state_file("absent.json");
    assert!(!path.exists(), "test precondition: state file must not exist");

    let config = AgentConfig {
        quadlet_dir: PathBuf::from("/etc/containers/systemd"),
        audit_dir: None,
        state_file: Some(path.clone()),
        reload_systemd: false,
        lock_path: Some(temp_state_file("agent_lock")),
    };

    let result = run_agent(&config).expect("run_agent should succeed");
    assert!(
        matches!(result, AgentExitReason::Uninitialized),
        "expected Uninitialized, got {:?}",
        result
    );
}

// run_agent with detached state returns Detached (clean exit with revision)
#[test]
fn agent_with_detached_state_returns_detached_exit_reason() {
    use core_ops::core::types::{
        ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState, TreeState,
    };
    use core_ops::core::reconcile::never_run_provenance;
    use core_ops::core::types::PERSISTED_PROVENANCE_SCHEMA_VERSION;
    use core_ops::io::state::write_persisted_state;

    let path = temp_state_file("detached.json");
    let state = PersistedProvenanceState {
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
        },
        reconciliation: never_run_provenance(),
        detached: true,
    };
    write_persisted_state(&path, &state).expect("write detached state");

    let config = AgentConfig {
        quadlet_dir: PathBuf::from("/etc/containers/systemd"),
        audit_dir: None,
        state_file: Some(path.clone()),
        reload_systemd: false,
        lock_path: Some(temp_state_file("agent_lock_detached")),
    };

    let result = run_agent(&config).expect("run_agent should succeed");
    assert!(
        matches!(result, AgentExitReason::Detached { .. }),
        "expected Detached for detached state, got {:?}",
        result
    );

    let _ = std::fs::remove_file(path);
}

// T030: agent with corrupt state returns error mentioning "corrupt", file path, and --force
#[test]
fn agent_with_corrupt_state_returns_distinct_error() {
    let path = temp_state_file("corrupt_agent.json");
    std::fs::write(&path, "not valid json").expect("write corrupt state");

    let config = AgentConfig {
        quadlet_dir: std::path::PathBuf::from("/etc/containers/systemd"),
        audit_dir: None,
        state_file: Some(path.clone()),
        reload_systemd: false,
        lock_path: Some(temp_state_file("agent_lock_corrupt")),
    };

    let err = run_agent(&config).expect_err("agent should fail on corrupt state");
    assert!(
        err.message.contains("corrupt"),
        "error should mention corrupt, got: {}",
        err.message
    );
    assert!(
        err.message.contains(path.to_str().unwrap()),
        "error should contain file path, got: {}",
        err.message
    );
    assert!(
        err.message.contains("--force"),
        "error should mention --force recovery, got: {}",
        err.message
    );

    let _ = std::fs::remove_file(path);
}
