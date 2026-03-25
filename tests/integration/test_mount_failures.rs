use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountDependency, MountVerificationMode, ObservedState, ObservedUnit, PathDependencyMode,
    QuadletType, RestartPolicy, UnitActiveState, UnitDependencyMode, Workload,
};

fn desired_state() -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: vec![Workload {
            name: "var-lib-immich-media".to_string(),
            quadlet_type: QuadletType::Mount,
            quadlet_contents: "[Mount]\nWhere=/var/lib/immich/media\n".to_string(),
            systemd_unit_name: "var-lib-immich-media.mount".to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        }],
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
            prepared_path: None,
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
    }
}

#[test]
fn invalid_mount_is_reported_as_blocked() {
    let desired = desired_state();
    let observed = ObservedState {
        observed_revision_id: None,
        units: vec![ObservedUnit {
            unit_name: "var-lib-immich-media.mount".to_string(),
            active_state: UnitActiveState::Inactive,
            enabled_state: EnabledState::Enabled,
        }],
        workloads: desired.workloads.clone(),
        last_reconcile_id: None,
        host_info: None,
    };
    let deps = ReconcileDependencies {
        load_desired: &|| Ok(desired.clone()),
        read_observed: &|_| Ok(observed.clone()),
        apply_plan: &|_, _| Ok(()),
    };

    let result = reconcile_apply(&deps).expect("reconcile apply");

    assert_eq!(result.run.summary, "mount blocked");
    assert!(result
        .verification_results
        .iter()
        .any(|result| result.details.as_deref().unwrap_or("").starts_with("blocked:")));
}

#[test]
fn recovery_after_mount_becomes_reachable_converges() {
    let desired = desired_state();
    let degraded = ObservedState {
        observed_revision_id: None,
        units: vec![ObservedUnit {
            unit_name: "var-lib-immich-media.mount".to_string(),
            active_state: UnitActiveState::Active,
            enabled_state: EnabledState::Enabled,
        }],
        workloads: desired.workloads.clone(),
        last_reconcile_id: None,
        host_info: None,
    };
    let healthy = degraded.clone();
    let mountinfo = std::env::temp_dir().join(format!(
        "core_ops_mount_recovery_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::write(
        &mountinfo,
        "36 25 0:32 / /var/lib/immich/media rw,relatime - nfs nas:/media rw\n",
    )
    .expect("write mountinfo");
    std::env::set_var("CORE_OPS_MOUNTINFO_PATH", &mountinfo);
    let _guard = MountInfoGuard;

    let calls = Cell::new(0usize);
    let deps = ReconcileDependencies {
        load_desired: &|| Ok(desired.clone()),
        read_observed: &|_| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Ok(degraded.clone())
            } else {
                Ok(healthy.clone())
            }
        },
        apply_plan: &|_, _| Ok(()),
    };

    let result = reconcile_apply(&deps).expect("reconcile apply");
    assert_eq!(result.run.summary, "converged mount-backed services");
}

struct MountInfoGuard;

impl Drop for MountInfoGuard {
    fn drop(&mut self) {
        std::env::remove_var("CORE_OPS_MOUNTINFO_PATH");
    }
}
use std::cell::Cell;
