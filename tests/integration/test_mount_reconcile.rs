use std::fs;
use std::path::{Path, PathBuf};

use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountDependency, MountVerificationMode, PathDependencyMode, PlanAction, PlanActionType,
    QuadletType, ReconciliationPlan, RestartPolicy, UnitDependencyMode, Workload,
};
use core_ops::io::apply::apply_plan_with_desired;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mount_management")
}

fn read_scenario(name: &str) -> String {
    fs::read_to_string(fixture_dir().join(name).join("scenario.yaml"))
        .expect("read mount reconcile scenario")
}

#[test]
fn reconcile_fixture_covers_invalid_and_busy_removal_paths() {
    let invalid = read_scenario("invalid-definition");
    let busy = read_scenario("busy-removal");

    assert!(invalid.contains("duplicate-target-path"));
    assert!(invalid.contains("invalid-ownership-boundary"));
    assert!(busy.contains("managed-removal"));
    assert!(busy.contains("busy-unmount-failure"));
    assert!(busy.contains("dependent-service-stop-first"));
}

#[test]
fn apply_prepares_target_path_and_starts_mount_before_service() {
    let temp = std::env::temp_dir().join(format!(
        "core_ops_mount_apply_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    fs::create_dir_all(&temp).expect("temp dir");
    let quadlet_dir = temp.join("quadlets");
    let systemd_dir = temp.join("systemd");
    fs::create_dir_all(&quadlet_dir).expect("quadlet dir");
    fs::create_dir_all(&systemd_dir).expect("systemd dir");
    std::env::set_var("CORE_OPS_SYSTEMD_UNIT_DIR", &systemd_dir);
    let _guard = EnvGuard;

    let log_path = temp.join("systemctl.log");
    write_systemctl_stub(&temp, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", temp.display(), old_path));
    let _path_guard = PathGuard(old_path);

    let target_path = temp.join("mnt/media");
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![
            Workload {
                name: "var-lib-immich-media".to_string(),
                quadlet_type: QuadletType::Mount,
                quadlet_contents: "[Mount]\nWhere=/var/lib/immich/media\n".to_string(),
                systemd_unit_name: "var-lib-immich-media.mount".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "immich".to_string(),
                quadlet_type: QuadletType::Container,
                quadlet_contents: "[Container]\nImage=immich\n".to_string(),
                systemd_unit_name: "immich.container".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
        ],
        mount_declarations: vec![MountDeclaration {
            id: "immich-media".to_string(),
            target_path: target_path.to_string_lossy().to_string(),
            source: "nas:/media".to_string(),
            fstype: "nfs".to_string(),
            mount_options: vec!["rw".to_string()],
            network_backed: true,
            automount: false,
            verification_mode: MountVerificationMode::UnitAndPath,
            ownership_scope: vec!["immich".to_string()],
            prepared_path: Some(core_ops::core::types::PreparedTargetPath {
                path: target_path.to_string_lossy().to_string(),
                create_if_missing: true,
                owner: None,
                group: None,
                mode: Some("0755".to_string()),
                service_consumed: true,
            }),
        }],
        mount_dependencies: vec![MountDependency {
            service_name: "immich".to_string(),
            mount_ids: vec!["immich-media".to_string()],
            consumed_paths: vec![target_path.to_string_lossy().to_string()],
            path_dependency_mode: PathDependencyMode::RequiresMountsFor,
            unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
        }],
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![
            action(PlanActionType::PreparePath, target_path.to_string_lossy().as_ref()),
            action(PlanActionType::WriteQuadlet, "var-lib-immich-media.mount"),
            action(PlanActionType::WriteQuadlet, "immich.container"),
            action(PlanActionType::ReloadSystemd, "var-lib-immich-media.mount"),
            action(PlanActionType::StartUnit, "var-lib-immich-media.mount"),
            action(PlanActionType::StartUnit, "immich.container"),
        ],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    apply_plan_with_desired(&plan, &desired, &quadlet_dir, true).expect("apply");

    assert!(target_path.exists());
    assert!(systemd_dir.join("var-lib-immich-media.mount").exists());
    let log_contents = fs::read_to_string(&log_path).expect("read log");
    let mount_start = log_contents
        .find("start var-lib-immich-media.mount")
        .expect("mount start");
    let service_start = log_contents.find("start immich.service").expect("service start");
    assert!(mount_start < service_start);
}

fn action(action_type: PlanActionType, target: &str) -> PlanAction {
    PlanAction {
        action_type,
        target: target.to_string(),
        preconditions: Vec::new(),
        postconditions: Vec::new(),
    }
}

fn write_systemctl_stub(dir: &Path, log_path: &Path) {
    let script = format!("#!/bin/sh\necho \"$@\" >> \"{}\"\nexit 0\n", log_path.display());
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

struct PathGuard(String);

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.0);
    }
}
