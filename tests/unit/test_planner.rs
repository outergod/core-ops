use std::collections::BTreeMap;

use core_ops::core::planner::{
    direct_and_transitive_prerequisite_refs, managed_object_ref, plan,
    plan_deterministic_reconciliation, plan_deterministic_reconciliation_with_runtime,
    plan_mount_units,
};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, DeterministicActionClass, EnabledState, Invariant,
    ManagedObjectKind, MountDeclaration, MountDependency, MountVerificationMode,
    NormalizedManagedObject, NormalizedSnapshot, ObservedState, PathDependencyMode, QuadletType,
    RestartPolicy, UnitDependencyMode, VerificationResult, VerificationStatus, Workload,
};
use core_ops::core::unit::{apply_service_mount_dependencies, render_mount_unit};

fn workload(name: &str) -> Workload {
    workload_with_type(name, QuadletType::Container)
}

fn workload_with_type(name: &str, quadlet_type: QuadletType) -> Workload {
    let extension = match quadlet_type {
        QuadletType::Container => "container",
        QuadletType::Socket => "socket",
        QuadletType::SocketDropIn => "socket-dropin",
        QuadletType::ConfigFile => "config",
        QuadletType::Mount => "mount",
        QuadletType::Automount => "automount",
        QuadletType::Volume => "volume",
        QuadletType::Pod => "pod",
        QuadletType::Network => "network",
    };
    Workload {
        name: name.to_string(),
        quadlet_type,
        quadlet_contents: "[Quadlet]".to_string(),
        systemd_unit_name: format!("{name}.{extension}"),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn desired_state(workloads: Vec<Workload>) -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
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

fn observed_state(workloads: Vec<Workload>) -> ObservedState {
    ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units: Vec::new(),
        workloads,
        last_reconcile_id: None,
        host_info: None,
    }
}

fn socket_dropin_workload(name: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type: QuadletType::SocketDropIn,
        quadlet_contents: "[Socket]".to_string(),
        systemd_unit_name: name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

#[test]
fn plan_is_deterministic_by_name_order() {
    let desired = desired_state(vec![workload("beta"), workload("alpha")]);
    let observed = observed_state(Vec::new());

    let plan = plan(&desired, &observed).expect("plan should succeed");

    let targets: Vec<String> = plan.actions.iter().map(|a| a.target.clone()).collect();

    let alpha_prefix = [
        "alpha.container".to_string(),
        "alpha.container".to_string(),
        "alpha.container".to_string(),
    ];
    assert_eq!(&targets[..3], &alpha_prefix[..]);
}

#[test]
fn plan_has_no_actions_when_states_match() {
    let workloads = vec![workload("alpha")];
    let desired = desired_state(workloads.clone());
    let observed = observed_state(workloads);

    let plan = plan(&desired, &observed).expect("plan should succeed");

    assert!(plan.actions.is_empty());
}

#[test]
fn plan_orders_actions_by_quadlet_type() {
    let desired = desired_state(vec![
        workload_with_type("socket", QuadletType::Socket),
        workload_with_type("container", QuadletType::Container),
        workload_with_type("network", QuadletType::Network),
        workload_with_type("volume", QuadletType::Volume),
    ]);
    let observed = observed_state(Vec::new());

    let plan = plan(&desired, &observed).expect("plan should succeed");
    let targets: Vec<String> = plan.actions.iter().map(|a| a.target.clone()).collect();

    let volume_prefix = ["volume.volume".to_string(), "volume.volume".to_string()];
    let container_prefix = [
        "container.container".to_string(),
        "container.container".to_string(),
        "container.container".to_string(),
    ];
    let network_prefix = [
        "network.network".to_string(),
        "network.network".to_string(),
        "network.network".to_string(),
    ];
    let socket_prefix = [
        "socket.socket".to_string(),
        "socket.socket".to_string(),
        "socket.socket".to_string(),
    ];

    assert_eq!(&targets[..2], &volume_prefix[..]);
    assert_eq!(&targets[2..5], &network_prefix[..]);
    assert_eq!(&targets[5..8], &container_prefix[..]);
    assert_eq!(&targets[8..11], &socket_prefix[..]);
}

#[test]
fn plan_restarts_socket_when_socket_dropin_removed() {
    let desired = desired_state(Vec::new());
    let observed = observed_state(vec![socket_dropin_workload("alpha.socket.d/10-host.conf")]);

    let plan = plan(&desired, &observed).expect("plan should succeed");

    let actions: Vec<_> = plan
        .actions
        .iter()
        .map(|a| (a.action_type.clone(), a.target.as_str()))
        .collect();
    assert!(actions.iter().any(|(action, target)| {
        *action == core_ops::core::types::PlanActionType::RemoveQuadlet
            && *target == "alpha.socket.d/10-host.conf"
    }));
    assert!(actions.iter().any(|(action, target)| {
        *action == core_ops::core::types::PlanActionType::ReloadSystemd
            && *target == "alpha.socket.d/10-host.conf"
    }));
    assert!(actions.iter().any(|(action, target)| {
        *action == core_ops::core::types::PlanActionType::RestartUnit && *target == "alpha.socket"
    }));
}

#[test]
fn mount_planning_expands_path_and_explicit_unit_dependencies() {
    let declaration = MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/var/lib/immich/media".to_string(),
        source: "nas:/volume1/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: vec!["rw".to_string()],
        network_backed: true,
        automount: true,
        verification_mode: MountVerificationMode::UnitAndPath,
        prepared_path: None,
    };
    let dependencies = vec![MountDependency {
        service_name: "immich".to_string(),
        mount_ids: vec!["immich-media".to_string()],
        consumed_paths: vec!["/var/lib/immich/media".to_string()],
        path_dependency_mode: PathDependencyMode::RequiresMountsFor,
        unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
    }];

    let planned = plan_mount_units(&declaration, &dependencies);

    assert_eq!(planned.mount_unit_name, "var-lib-immich-media.mount");
    assert_eq!(
        planned.automount_unit_name.as_deref(),
        Some("var-lib-immich-media.automount")
    );
    assert_eq!(
        planned.service_dependency_edits[0].requires_mounts_for,
        vec!["/var/lib/immich/media".to_string()]
    );
    assert_eq!(
        planned.service_dependency_edits[0].after_units,
        vec![
            "var-lib-immich-media.automount".to_string(),
            "var-lib-immich-media.mount".to_string()
        ]
    );
    assert_eq!(
        planned.removal_candidates,
        vec![
            "var-lib-immich-media.automount".to_string(),
            "var-lib-immich-media.mount".to_string()
        ]
    );

    let rendered = render_mount_unit(&declaration);
    assert_eq!(rendered.0, "var-lib-immich-media.mount");
    let service_contents = apply_service_mount_dependencies(
        "[Unit]\nDescription=Immich\n",
        &planned.service_dependency_edits[0],
    );
    assert!(service_contents.contains("RequiresMountsFor=/var/lib/immich/media"));
    assert!(service_contents
        .contains("After=var-lib-immich-media.automount var-lib-immich-media.mount"));
}

fn normalized_object(
    object_id: &str,
    object_kind: ManagedObjectKind,
    material_fields: &[(&str, &str)],
    dependency_refs: &[&str],
) -> NormalizedManagedObject {
    NormalizedManagedObject {
        object_id: object_id.to_string(),
        object_kind,
        material_fields: material_fields
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
        dependency_refs: dependency_refs
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

#[test]
fn deterministic_planner_orders_objects_by_dependency_graph() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service")],
                &["config:/etc/alpha/env", "var-lib-alpha.mount"],
            ),
            normalized_object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env")],
                &[],
            ),
            normalized_object(
                "var-lib-alpha.mount",
                ManagedObjectKind::Mount,
                &[("unit", "var-lib-alpha.mount")],
                &[],
            ),
        ],
    };
    let actual = desired.clone();
    let applied = desired.clone();

    let plan = plan_deterministic_reconciliation(&desired, Some(&applied), &actual)
        .expect("deterministic plan");
    let ordered: Vec<&str> = plan
        .actions
        .iter()
        .map(|action| action.object_id.as_str())
        .collect();

    assert_eq!(
        ordered,
        vec![
            "config:/etc/alpha/env",
            "var-lib-alpha.mount",
            "alpha.service"
        ]
    );
}

#[test]
fn canonical_object_identity_and_dependency_depth_are_derived_consistently() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "config:/etc/alpha/base",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/base")],
                &[],
            ),
            normalized_object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env")],
                &["config:/etc/alpha/base"],
            ),
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "ghcr.io/example:v2")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };
    let actual = desired.clone();
    let result = plan_deterministic_reconciliation(&desired, Some(&desired), &actual)
        .expect("deterministic plan");

    let object_ref = managed_object_ref("alpha.service", &ManagedObjectKind::GeneratedUnit);
    let (direct, transitive) =
        direct_and_transitive_prerequisite_refs(&result.graph, "alpha.service");

    assert_eq!(object_ref.display_id, "service/alpha.service");
    assert_eq!(direct.len(), 1);
    assert_eq!(direct[0].display_id, "config/etc/alpha/env");
    assert_eq!(transitive.len(), 1);
    assert_eq!(transitive[0].display_id, "config/etc/alpha/base");
}

#[test]
fn deterministic_planner_classifies_create_update_delete_and_blocked_actions() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "stable")],
                &[],
            ),
            normalized_object(
                "beta.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "beta.service"), ("image", "stable")],
                &[],
            ),
            normalized_object(
                "gamma.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "gamma.service")],
                &["missing.dependency"],
            ),
        ],
    };
    let applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "stable")],
                &[],
            ),
            normalized_object(
                "beta.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "beta.service"), ("image", "stable")],
                &[],
            ),
            normalized_object(
                "obsolete.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "obsolete.service")],
                &[],
            ),
        ],
    };
    let actual = NormalizedSnapshot {
        revision_id: Some("obs-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "debug")],
                &[],
            ),
            normalized_object(
                "obsolete.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "obsolete.service")],
                &[],
            ),
        ],
    };

    let plan = plan_deterministic_reconciliation(&desired, Some(&applied), &actual)
        .expect("deterministic plan");
    let classifications: Vec<(&str, DeterministicActionClass)> = plan
        .actions
        .iter()
        .map(|action| (action.object_id.as_str(), action.classification.clone()))
        .collect();

    assert!(classifications.contains(&("alpha.service", DeterministicActionClass::Update)));
    assert!(classifications.contains(&("beta.service", DeterministicActionClass::Create)));
    assert!(classifications.contains(&("gamma.service", DeterministicActionClass::Blocked)));
    assert!(classifications.contains(&("obsolete.service", DeterministicActionClass::Delete)));
}

#[test]
fn deterministic_planner_rejects_semantic_dependency_cycles() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-3".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service")],
                &["beta.service"],
            ),
            normalized_object(
                "beta.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "beta.service")],
                &["alpha.service"],
            ),
        ],
    };
    let actual = NormalizedSnapshot {
        revision_id: Some("obs-3".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: Vec::new(),
    };

    let err = plan_deterministic_reconciliation(&desired, None, &actual)
        .expect_err("cycle must fail planning");

    assert_eq!(err.class, core_ops::core::types::FailureClass::Validation);
    assert!(err.message.contains("semantic dependency cycle"));
}

#[test]
fn deterministic_planner_deletes_in_reverse_dependency_order() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-4".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: Vec::new(),
    };
    let actual = NormalizedSnapshot {
        revision_id: Some("obs-4".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env")],
                &[],
            ),
            normalized_object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };

    let plan =
        plan_deterministic_reconciliation(&desired, None, &actual).expect("deterministic plan");
    let delete_ids: Vec<&str> = plan
        .actions
        .iter()
        .filter(|action| action.classification == DeterministicActionClass::Delete)
        .map(|action| action.object_id.as_str())
        .collect();

    assert_eq!(delete_ids, vec!["alpha.service", "config:/etc/alpha/env"]);
}

#[test]
fn deterministic_planner_uses_restart_for_dependency_driven_reactivation() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "config:/etc/app.env",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "DB_HOST=new")],
                &[],
            ),
            normalized_object(
                "app.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.service"), ("image", "stable")],
                &["config:/etc/app.env"],
            ),
        ],
    };
    let applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "config:/etc/app.env",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "DB_HOST=old")],
                &[],
            ),
            normalized_object(
                "app.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.service"), ("image", "stable")],
                &["config:/etc/app.env"],
            ),
        ],
    };
    let actual = NormalizedSnapshot {
        revision_id: Some("obs-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                "config:/etc/app.env",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "DB_HOST=old")],
                &[],
            ),
            normalized_object(
                "app.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.service"), ("image", "stable")],
                &["config:/etc/app.env"],
            ),
        ],
    };

    let plan = plan_deterministic_reconciliation(&desired, Some(&applied), &actual)
        .expect("deterministic plan");
    let actions = plan
        .actions
        .iter()
        .map(|action| {
            (
                action.object_id.as_str(),
                action.classification.clone(),
                action.reason.clone(),
            )
        })
        .collect::<Vec<_>>();

    assert!(actions.iter().any(|(object_id, classification, _)| {
        *object_id == "config:/etc/app.env" && *classification == DeterministicActionClass::Update
    }));
    assert!(actions.iter().any(|(object_id, classification, reason)| {
        *object_id == "app.service"
            && *classification == DeterministicActionClass::Restart
            && reason.contains("config:/etc/app.env changed")
    }));
}

#[test]
fn deterministic_planner_uses_recover_for_runtime_variance_without_declarative_drift() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![normalized_object(
            "app.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "app.service"), ("image", "stable")],
            &[],
        )],
    };
    let applied = desired.clone();
    let actual = desired.clone();
    let verification_results = vec![VerificationResult {
        target: "app.service".to_string(),
        status: VerificationStatus::Failure,
        details: Some("unit not active: failed".to_string()),
    }];

    let plan = plan_deterministic_reconciliation_with_runtime(
        &desired,
        Some(&applied),
        &actual,
        &verification_results,
    )
    .expect("deterministic plan");
    let action = plan
        .actions
        .iter()
        .find(|action| action.object_id == "app.service")
        .unwrap();

    assert_eq!(action.classification, DeterministicActionClass::Recover);
    assert!(action.reason.contains("runtime reconciliation required"));
    assert!(plan.drift_records.iter().any(|record| {
        record.object_id == "app.service"
            && record.category == core_ops::core::types::DriftCategory::RuntimeVariance
    }));
}

#[test]
fn deterministic_planner_classifies_restart_when_config_dependency_is_removed() {
    // When a config file that a workload depends on is removed from desired,
    // the workload must be classified Restart (not NoOp). The desired
    // dependency_refs omit the removed path because managed_config_paths no
    // longer includes it, but the desired quadlet contents still contain an
    // EnvironmentFile= reference — the authoritative evidence the planner uses.
    let config_path = "/etc/runner/env";
    let quadlet_contents =
        "[Container]\nImage=docker.io/example/app:v1\nEnvironmentFile=/etc/runner/env\n";

    // Desired: container only — config file has been removed from managed_config_paths,
    // but the quadlet still references it via EnvironmentFile=.
    // dependency_refs is empty because managed_config_paths no longer lists config_path.
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![normalized_object(
            "app.container",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "app.container"), ("contents", quadlet_contents)],
            &[], // config removed from managed_config_paths → no dep ref in desired
        )],
    };
    // Applied: both config file and container present; container depends on config.
    let applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                config_path,
                ManagedObjectKind::RenderedArtifact,
                &[("target_path", config_path)],
                &[],
            ),
            normalized_object(
                "app.container",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.container")],
                &[config_path], // dep recorded at last-apply time
            ),
        ],
    };
    // Actual: config file still present on disk; workload material_fields match
    // desired (container itself unchanged) so classification is NoOp until the
    // config-delete restart pass promotes it to Restart.
    // dependency_refs are empty because they're re-derived from the current desired
    // state (managed_config_paths no longer includes config_path).
    let actual = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                config_path,
                ManagedObjectKind::RenderedArtifact,
                &[("target_path", config_path)],
                &[],
            ),
            normalized_object(
                "app.container",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.container"), ("contents", quadlet_contents)],
                &[], // same as desired — derived from current managed_config_paths
            ),
        ],
    };

    let plan = plan_deterministic_reconciliation(&desired, Some(&applied), &actual)
        .expect("deterministic plan");

    let app_action = plan
        .actions
        .iter()
        .find(|a| a.object_id == "app.container")
        .expect("app.container action present");

    assert_eq!(
        app_action.classification,
        DeterministicActionClass::Restart,
        "workload must be Restart when its config dependency is deleted, got {:?}",
        app_action.classification
    );
    assert!(
        app_action.reason.contains(config_path),
        "reason must reference the deleted config path"
    );
}

#[test]
fn deterministic_planner_classifies_restart_when_config_dependency_is_removed_without_baseline() {
    // Same scenario as the previous test but with last_applied = None (first reconcile
    // after state reset). applied_map is empty, so the pass must fall back to scanning
    // the actual workload's raw quadlet contents for EnvironmentFile= references.
    let config_path = "/etc/runner/env";

    let quadlet_contents = "[Container]\nImage=docker.io/example/app:v1\nEnvironmentFile=/etc/runner/env\n";

    // Desired: container only — config file removed, no managed_config_paths entry.
    // Material fields match actual (container itself unchanged) so classification is
    // NoOp until the config-delete restart pass promotes it to Restart.
    // dependency_refs is empty because managed_config_paths no longer includes the path.
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![normalized_object(
            "app.container",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "app.container"), ("contents", quadlet_contents)],
            &[],
        )],
    };
    // Actual: config file still on disk; workload quadlet contents reference it via
    // EnvironmentFile= but dependency_refs is empty (derived from current desired).
    let actual = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            normalized_object(
                config_path,
                ManagedObjectKind::RenderedArtifact,
                &[("target_path", config_path)],
                &[],
            ),
            normalized_object(
                "app.container",
                ManagedObjectKind::GeneratedUnit,
                // Same contents as desired so the container itself is NoOp —
                // only the config deletion should trigger the Restart classification.
                // The EnvironmentFile= line is the fallback evidence used when
                // no applied snapshot exists (dependency_refs is empty).
                &[("unit", "app.container"), ("contents", quadlet_contents)],
                &[], // dependency_refs empty — current desired has no managed_config_paths
            ),
        ],
    };

    // No baseline — first reconcile after state reset.
    let plan = plan_deterministic_reconciliation(&desired, None, &actual)
        .expect("deterministic plan");

    let app_action = plan
        .actions
        .iter()
        .find(|a| a.object_id == "app.container")
        .expect("app.container action present");

    assert_eq!(
        app_action.classification,
        DeterministicActionClass::Restart,
        "workload must be Restart when its config dep is deleted and no baseline exists, got {:?}",
        app_action.classification
    );
    assert!(
        app_action.reason.contains(config_path),
        "reason must reference the deleted config path"
    );
}
