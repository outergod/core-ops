use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountVerificationMode, ObservedState, ObservedUnit, PathDependencyMode, QuadletType,
    RestartPolicy, ServiceDependencyEdit, UnitActiveState, UnitDependencyMode,
    VerificationStatus, Workload,
};
use core_ops::core::unit::{apply_service_mount_dependencies, render_automount_unit, render_mount_unit};
use core_ops::core::verify::verify_state;

fn workload(name: &str, quadlet_type: QuadletType, unit: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type,
        quadlet_contents: "[Unit]".to_string(),
        systemd_unit_name: unit.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn desired_state(workloads: Vec<Workload>) -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads,
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn observed_state(units: Vec<ObservedUnit>) -> ObservedState {
    ObservedState {
        observed_revision_id: None,
        units,
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: None,
    }
}

#[test]
fn verify_container_requires_active_unit() {
    let desired = desired_state(vec![workload("alpha", QuadletType::Container, "alpha.container")]);
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "alpha.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Enabled,
    }]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, VerificationStatus::Failure);
}

#[test]
fn verify_volume_accepts_loaded_unit() {
    let desired = desired_state(vec![workload("gamma", QuadletType::Volume, "gamma.volume")]);
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "gamma-volume.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Disabled,
    }]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, VerificationStatus::Success);
}

#[test]
fn render_mount_dependencies_into_service_unit() {
    let edit = ServiceDependencyEdit {
        service_name: "immich".to_string(),
        requires_mounts_for: vec!["/var/lib/immich/media".to_string()],
        after_units: vec!["var-lib-immich-media.mount".to_string()],
        requires_units: vec!["var-lib-immich-media.mount".to_string()],
    };

    let rendered = apply_service_mount_dependencies("[Container]\nImage=immich\n", &edit);

    assert!(rendered.contains("RequiresMountsFor=/var/lib/immich/media"));
    assert!(rendered.contains("After=var-lib-immich-media.mount"));
    assert!(rendered.contains("Requires=var-lib-immich-media.mount"));
}

#[test]
fn verify_mount_requires_active_unit_and_mounted_target() {
    let mount = MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/var/lib/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: vec!["rw".to_string()],
        network_backed: true,
        automount: false,
        verification_mode: MountVerificationMode::UnitAndPath,
        ownership_scope: vec!["immich".to_string()],
        prepared_path: None,
    };
    let mount_workload = workload(
        "var-lib-immich-media",
        QuadletType::Mount,
        "var-lib-immich-media.mount",
    );
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![mount_workload],
        mount_declarations: vec![mount.clone()],
        mount_dependencies: vec![core_ops::core::types::MountDependency {
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
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "var-lib-immich-media.mount".to_string(),
        active_state: UnitActiveState::Active,
        enabled_state: EnabledState::Enabled,
    }]);

    let temp = std::env::temp_dir().join(format!(
        "core_ops_mountinfo_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::write(
        &temp,
        "36 25 0:32 / /var/lib/immich/media rw,relatime - nfs nas:/media rw\n",
    )
    .expect("write mountinfo");
    std::env::set_var("CORE_OPS_MOUNTINFO_PATH", &temp);
    let _guard = MountInfoGuard;

    let rendered_mount = render_mount_unit(&mount);
    assert_eq!(rendered_mount.0, "var-lib-immich-media.mount");

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, VerificationStatus::Success);
}

#[test]
fn render_automount_unit_and_verify_active_automount() {
    let mount = MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/srv/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: vec!["rw".to_string()],
        network_backed: true,
        automount: true,
        verification_mode: MountVerificationMode::UnitAndPath,
        ownership_scope: vec!["immich".to_string()],
        prepared_path: None,
    };
    let automount_workload = workload(
        "srv-immich-media",
        QuadletType::Automount,
        "srv-immich-media.automount",
    );
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![automount_workload],
        mount_declarations: vec![mount.clone()],
        mount_dependencies: vec![core_ops::core::types::MountDependency {
            service_name: "immich".to_string(),
            mount_ids: vec!["immich-media".to_string()],
            consumed_paths: vec!["/srv/immich/media".to_string()],
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
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "srv-immich-media.automount".to_string(),
        active_state: UnitActiveState::Active,
        enabled_state: EnabledState::Enabled,
    }]);

    let rendered_automount = render_automount_unit(&mount).expect("automount unit");
    assert_eq!(rendered_automount.0, "srv-immich-media.automount");

    let edit = ServiceDependencyEdit {
        service_name: "immich".to_string(),
        requires_mounts_for: vec!["/srv/immich/media".to_string()],
        after_units: vec![
            "srv-immich-media.automount".to_string(),
            "srv-immich-media.mount".to_string(),
        ],
        requires_units: vec![
            "srv-immich-media.automount".to_string(),
            "srv-immich-media.mount".to_string(),
        ],
    };
    let rendered_service = apply_service_mount_dependencies("[Container]\nImage=immich\n", &edit);
    assert!(rendered_service.contains("Requires=srv-immich-media.automount srv-immich-media.mount"));

    let results = verify_state(&desired, &observed);
    assert_eq!(results[0].status, VerificationStatus::Success);
}

#[test]
fn verify_automount_backed_mount_accepts_inactive_mount_when_automount_is_active() {
    let mount = MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/srv/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: vec!["rw".to_string()],
        network_backed: true,
        automount: true,
        verification_mode: MountVerificationMode::UnitAndPath,
        ownership_scope: vec!["immich".to_string()],
        prepared_path: None,
    };
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![
            workload(
                "srv-immich-media",
                QuadletType::Mount,
                "srv-immich-media.mount",
            ),
            workload(
                "srv-immich-media",
                QuadletType::Automount,
                "srv-immich-media.automount",
            ),
        ],
        mount_declarations: vec![mount],
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };
    let observed = observed_state(vec![
        ObservedUnit {
            unit_name: "srv-immich-media.mount".to_string(),
            active_state: UnitActiveState::Inactive,
            enabled_state: EnabledState::Enabled,
        },
        ObservedUnit {
            unit_name: "srv-immich-media.automount".to_string(),
            active_state: UnitActiveState::Active,
            enabled_state: EnabledState::Enabled,
        },
    ]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| result.status == VerificationStatus::Success));
}

struct MountInfoGuard;

impl Drop for MountInfoGuard {
    fn drop(&mut self) {
        std::env::remove_var("CORE_OPS_MOUNTINFO_PATH");
    }
}
