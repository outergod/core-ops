use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, QuadletType, RestartPolicy,
    Workload,
};
use core_ops::io::observed::read_observed_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{prefix}_{nanos}"));
    path
}

fn socket_dropin_workload(name: &str, contents: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type: QuadletType::SocketDropIn,
        quadlet_contents: contents.to_string(),
        systemd_unit_name: name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

#[test]
fn observed_state_ignores_unmanaged_socket_dropins() {
    let _lock = path_lock().lock().expect("path lock");
    let quadlet_dir = temp_dir("core_ops_socket_dropins_quadlets");
    std::fs::create_dir_all(&quadlet_dir).expect("create quadlet dir");

    let systemd_dir = temp_dir("core_ops_socket_dropins_systemd");
    std::fs::create_dir_all(&systemd_dir).expect("create systemd dir");
    std::env::set_var("CORE_OPS_SYSTEMD_UNIT_DIR", &systemd_dir);
    let _guard = EnvGuard {
        key: "CORE_OPS_SYSTEMD_UNIT_DIR",
    };

    let socket_path = systemd_dir.join("alpha.socket");
    let socket_contents = "# managed-by: core-ops\n[Socket]\nListenStream=127.0.0.1:8080\n";
    std::fs::write(&socket_path, socket_contents).expect("write socket");

    let dropin_dir = systemd_dir.join("alpha.socket.d");
    std::fs::create_dir_all(&dropin_dir).expect("create dropin dir");
    let known_path = dropin_dir.join("10-known.conf");
    let unknown_path = dropin_dir.join("20-unknown.conf");
    std::fs::write(&known_path, "Known=1").expect("write known dropin");
    std::fs::write(&unknown_path, "Unknown=1").expect("write unknown dropin");

    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![socket_dropin_workload(
            "alpha.socket.d/10-known.conf",
            "Known=1",
        )],
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };

    let observed = read_observed_state(&quadlet_dir, Some(&desired), Some("obs".to_string()))
        .expect("observed");

    let names: Vec<_> = observed
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.as_str())
        .collect();
    assert!(names.contains(&"alpha.socket.d/10-known.conf"));
    assert!(!names.contains(&"alpha.socket.d/20-unknown.conf"));
}

struct EnvGuard {
    key: &'static str,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.key);
    }
}
