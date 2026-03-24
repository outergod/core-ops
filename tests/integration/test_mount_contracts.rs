use std::fs;
use std::path::{Path, PathBuf};

use core_ops::core::planner::plan;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountDependency, MountVerificationMode, ObservedState, PathDependencyMode, QuadletType,
    RestartPolicy, UnitDependencyMode, Workload,
};
use core_ops::cli::report::format_plan_report;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mount_management")
}

fn read_scenario(name: &str) -> String {
    fs::read_to_string(fixture_dir().join(name).join("scenario.yaml"))
        .expect("read mount management scenario")
}

#[test]
fn mount_management_fixture_scenarios_exist() {
    let dir = fixture_dir();
    assert!(dir.join("README.md").exists());
    assert!(dir.join("normal-nfs/scenario.yaml").exists());
    assert!(dir.join("network-automount/scenario.yaml").exists());
    assert!(dir.join("invalid-definition/scenario.yaml").exists());
    assert!(dir.join("busy-removal/scenario.yaml").exists());
}

#[test]
fn contract_fixture_covers_normal_and_automount_dependency_semantics() {
    let normal = read_scenario("normal-nfs");
    let automount = read_scenario("network-automount");

    assert!(normal.contains("named-mount-declaration"));
    assert!(normal.contains("requires-mounts-for"));
    assert!(automount.contains("automount-enabled"));
    assert!(automount.contains("explicit-unit-dependencies"));
    assert!(automount.contains("path-based-dependencies"));
}

#[test]
fn plan_includes_mount_units_dependency_semantics_and_prepare_path_actions() {
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![
            Workload {
                name: "immich".to_string(),
                quadlet_type: QuadletType::Container,
                quadlet_contents: "[Container]\nImage=immich\n[Service]\nRequiresMountsFor=/var/lib/immich/media\n[Unit]\nAfter=var-lib-immich-media.mount\nRequires=var-lib-immich-media.mount\n".to_string(),
                systemd_unit_name: "immich.container".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "var-lib-immich-media".to_string(),
                quadlet_type: QuadletType::Mount,
                quadlet_contents: "[Mount]\nWhere=/var/lib/immich/media\n".to_string(),
                systemd_unit_name: "var-lib-immich-media.mount".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
        ],
        mount_declarations: vec![MountDeclaration {
            id: "immich-media".to_string(),
            target_path: "/var/lib/immich/media".to_string(),
            source: "nas:/media".to_string(),
            fstype: "nfs".to_string(),
            mount_options: vec!["rw".to_string()],
            network_backed: true,
            automount: false,
            verification_mode: MountVerificationMode::UnitAndPath,
            ownership_scope: vec!["immich".to_string()],
            prepared_path: Some(core_ops::core::types::PreparedTargetPath {
                path: "/var/lib/immich/media".to_string(),
                create_if_missing: true,
                owner: None,
                group: None,
                mode: None,
                service_consumed: true,
            }),
        }],
        mount_dependencies: vec![MountDependency {
            service_name: "immich".to_string(),
            mount_ids: vec!["immich-media".to_string()],
            consumed_paths: vec!["/var/lib/immich/media".to_string()],
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
    let observed = ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units: Vec::new(),
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: None,
    };

    let plan = plan(&desired, &observed).expect("plan");
    let report = format_plan_report(&plan, &core_ops::core::diff::diff_workloads(&desired.workloads, &observed.workloads));

    assert!(desired.workloads[0].quadlet_contents.contains("RequiresMountsFor=/var/lib/immich/media"));
    assert!(desired.workloads[0].quadlet_contents.contains("After=var-lib-immich-media.mount"));
    assert!(report.contains("PreparePath: /var/lib/immich/media"));
    assert!(report.contains("WriteUnit: var-lib-immich-media.mount"));
}
