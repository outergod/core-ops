use std::fs;

use crate::integration::env_lock::path_lock;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, PlanAction, PlanActionType,
    QuadletType, ReconciliationPlan, RestartPolicy, Workload,
};
use core_ops::io::apply::apply_plan;

#[test]
fn managed_mount_removal_fails_when_target_is_still_busy() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let temp = std::env::temp_dir().join(format!(
        "core_ops_mount_remove_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&temp).expect("temp dir");
    let systemd_dir = temp.join("systemd");
    let quadlet_dir = temp.join("quadlets");
    fs::create_dir_all(&systemd_dir).expect("systemd dir");
    fs::create_dir_all(&quadlet_dir).expect("quadlet dir");
    std::env::set_var("CORE_OPS_SYSTEMD_UNIT_DIR", &systemd_dir);
    let _guard = EnvGuard;

    fs::write(
        systemd_dir.join("var-lib-immich-media.mount"),
        "[Mount]\nWhere=/var/lib/immich/media\n",
    )
    .expect("write mount unit");
    let mountinfo = temp.join("mountinfo");
    fs::write(
        &mountinfo,
        "36 25 0:32 / /var/lib/immich/media rw,relatime - nfs nas:/media rw\n",
    )
    .expect("write mountinfo");
    std::env::set_var("CORE_OPS_MOUNTINFO_PATH", &mountinfo);
    let _mountinfo_guard = MountInfoGuard;

    let old_path = std::env::var("PATH").unwrap_or_default();
    let log_path = temp.join("systemctl.log");
    write_systemctl_stub(&temp, &log_path);
    std::env::set_var("PATH", format!("{}:{}", temp.display(), old_path));
    let _path_guard = PathGuard(old_path);

    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![Workload {
            name: "immich".to_string(),
            quadlet_type: QuadletType::Container,
            quadlet_contents: "[Container]\nImage=immich\n".to_string(),
            systemd_unit_name: "immich.container".to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        }],
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };
    let plan = ReconciliationPlan {
        plan_id: "plan:remove".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: Some("obs".to_string()),
        actions: vec![PlanAction {
            action_type: PlanActionType::RemoveQuadlet,
            target: "var-lib-immich-media.mount".to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let err = match apply_plan(&plan, &desired.workloads, &quadlet_dir, false) {
        Ok(_) => panic!("expected busy mount removal failure"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("busy mount removal"));
    let log_contents = fs::read_to_string(&log_path).expect("read log");
    assert!(log_contents.contains("stop immich.service"));
}

fn write_systemctl_stub(dir: &std::path::Path, log_path: &std::path::Path) {
    let script = format!(
        "#!/bin/sh\necho \"$@\" >> \"{}\"\nexit 0\n",
        log_path.display()
    );
    let path = dir.join("systemctl");
    fs::write(&path, script).expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod");
    }
}

struct EnvGuard;
impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var("CORE_OPS_SYSTEMD_UNIT_DIR");
    }
}

struct MountInfoGuard;
impl Drop for MountInfoGuard {
    fn drop(&mut self) {
        std::env::remove_var("CORE_OPS_MOUNTINFO_PATH");
    }
}

struct PathGuard(String);
impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.0);
    }
}
