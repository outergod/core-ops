use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::core::planner::plan;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountVerificationMode, QuadletType, RestartPolicy, Workload,
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
fn removes_stale_config_files_under_managed_root() {
    let quadlet_dir = temp_dir("core_ops_config_cleanup_quadlets");
    std::fs::create_dir_all(&quadlet_dir).expect("create quadlet dir");

    let root = temp_dir("core_ops_config_root").join("etc/service");
    std::fs::create_dir_all(&root).expect("create config root");

    let keep_path = root.join("keep.conf");
    let stale_path = root.join("stale.conf");
    std::fs::write(&keep_path, "keep").expect("write keep");
    std::fs::write(&stale_path, "stale").expect("write stale");

    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![config_workload(
            keep_path.to_string_lossy().as_ref(),
            "keep",
        )],
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
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
    assert!(targets.contains(&stale_path.to_string_lossy().as_ref()));
    assert!(!targets.contains(&keep_path.to_string_lossy().as_ref()));
}

#[test]
fn mount_related_config_reapply_is_idempotent() {
    let _lock = path_lock().lock().expect("path lock");
    let quadlet_dir = temp_dir("core_ops_mount_config_quadlets");
    let systemd_dir = temp_dir("core_ops_mount_config_systemd");
    std::fs::create_dir_all(&quadlet_dir).expect("create quadlet dir");
    std::fs::create_dir_all(&systemd_dir).expect("create systemd dir");

    let previous_unit_dir = std::env::var_os("CORE_OPS_SYSTEMD_UNIT_DIR");
    std::env::set_var("CORE_OPS_SYSTEMD_UNIT_DIR", &systemd_dir);
    let _unit_dir_guard = SystemdUnitDirGuard(previous_unit_dir);

    let mount_root = temp_dir("core_ops_mount_root").join("media");
    std::fs::create_dir_all(&mount_root).expect("create mount root");
    let config_path = mount_root.join("immich.env");
    let config_contents = "IMMICH_MEDIA_DIR=/srv/immich/media\n";
    std::fs::write(&config_path, config_contents).expect("write config");

    let mount_unit_contents = "[Mount]\nWhere=/srv/immich/media\nWhat=nas:/media\nType=nfs\n";
    std::fs::write(
        systemd_dir.join("srv-immich-media.mount"),
        mount_unit_contents,
    )
    .expect("write mount unit");

    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![
            Workload {
                name: "srv-immich-media".to_string(),
                quadlet_type: QuadletType::Mount,
                quadlet_contents: mount_unit_contents.to_string(),
                systemd_unit_name: "srv-immich-media.mount".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            config_workload(config_path.to_string_lossy().as_ref(), config_contents),
        ],
        mount_declarations: vec![MountDeclaration {
            id: "immich-media".to_string(),
            target_path: "/srv/immich/media".to_string(),
            source: "nas:/media".to_string(),
            fstype: "nfs".to_string(),
            mount_options: Vec::new(),
            network_backed: true,
            automount: false,
            verification_mode: MountVerificationMode::UnitAndPath,
            prepared_path: None,
        }],
        mount_dependencies: Vec::new(),
        managed_config_paths: vec![config_path.to_string_lossy().to_string()],
        managed_config_roots: vec![mount_root.to_string_lossy().to_string()],
        invariants: vec![
            Invariant::BoundariesDeclared,
            Invariant::DeterministicPlan,
            Invariant::IdempotentApply,
        ],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };

    let observed = read_observed_state(&quadlet_dir, Some(&desired), Some("obs".to_string()))
        .expect("observed");
    let plan = plan(&desired, &observed).expect("plan");

    assert!(
        plan.actions.is_empty(),
        "expected idempotent reapply, got actions: {:?}",
        plan.actions
    );
}

struct SystemdUnitDirGuard(Option<std::ffi::OsString>);

impl Drop for SystemdUnitDirGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            std::env::set_var("CORE_OPS_SYSTEMD_UNIT_DIR", value);
        } else {
            std::env::remove_var("CORE_OPS_SYSTEMD_UNIT_DIR");
        }
    }
}
