use std::fs;
use std::path::PathBuf;

use core_ops::core::types::{
    EnabledState, PlanAction, PlanActionType, ReconciliationPlan, RestartPolicy, Workload,
};
use core_ops::io::apply::apply_plan;

use crate::integration::env_lock::path_lock;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn write_systemctl_stub(dir: &PathBuf, log_path: &PathBuf) -> PathBuf {
    let bin_path = dir.join("systemctl");
    let script = format!(
        "#!/bin/sh\n\n\
echo \"$@\" >> \"{}\"\n\
exit 1\n",
        log_path.display()
    );
    fs::write(&bin_path, script).expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
    bin_path
}

#[test]
fn apply_skips_enable_disable_for_generated_units() {
    let _lock = path_lock().lock().expect("path lock");
    let temp = temp_dir("core_ops_no_enable_disable");
    fs::create_dir_all(&temp).expect("temp dir");

    let log_path = temp.join("systemctl.log");
    write_systemctl_stub(&temp, &log_path);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let quadlet_dir = temp.join("quadlets");
    fs::create_dir_all(&quadlet_dir).expect("quadlet dir");

    let workload = Workload {
        name: "alpha".to_string(),
        quadlet_type: core_ops::core::types::QuadletType::Container,
        quadlet_contents: "[Container]\nImage=alpine".to_string(),
        systemd_unit_name: "alpha.container".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };

    let plan = ReconciliationPlan {
        plan_id: "plan:no-enable-disable".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![
            action(PlanActionType::WriteQuadlet, "alpha.container"),
            action(PlanActionType::EnableUnit, "alpha.container"),
            action(PlanActionType::DisableUnit, "alpha.container"),
        ],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let result = apply_plan(&plan, &[workload], &quadlet_dir, true);
    assert!(result.is_ok());
    assert!(!log_path.exists(), "systemctl should not be invoked");
}

fn action(action_type: PlanActionType, target: &str) -> PlanAction {
    PlanAction {
        action_type,
        target: target.to_string(),
        preconditions: Vec::new(),
        postconditions: Vec::new(),
    }
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}
