use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::core::planner::plan;
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

fn config_workload(path: &str, contents: &str) -> Workload {
    Workload {
        name: path.to_string(),
        quadlet_type: QuadletType::ConfigFile,
        quadlet_contents: contents.to_string(),
        systemd_unit_name: path.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

#[test]
fn does_not_manage_files_outside_config_roots() {
    let quadlet_dir = temp_dir("core_ops_config_roots_quadlets");
    std::fs::create_dir_all(&quadlet_dir).expect("create quadlet dir");

    let root = temp_dir("core_ops_config_root").join("etc/service");
    std::fs::create_dir_all(&root).expect("create config root");

    let keep_path = root.join("keep.conf");
    std::fs::write(&keep_path, "keep").expect("write keep");

    let outside_path = temp_dir("core_ops_unmanaged").join("unmanaged.conf");
    std::fs::write(&outside_path, "outside").expect("write outside");

    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![config_workload(
            keep_path.to_string_lossy().as_ref(),
            "keep",
        )],
        managed_config_paths: vec![keep_path.to_string_lossy().to_string()],
        managed_config_roots: vec![root.to_string_lossy().to_string()],
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };

    let observed = read_observed_state(&quadlet_dir, Some(&desired), Some("obs".to_string()))
        .expect("observed");
    let plan = plan(&desired, &observed).expect("plan");

    let targets: Vec<_> = plan.actions.iter().map(|a| a.target.as_str()).collect();
    assert!(!targets.contains(&outside_path.to_string_lossy().as_ref()));
}
