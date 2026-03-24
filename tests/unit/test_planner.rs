use core_ops::core::planner::plan;
use core_ops::core::planner::plan_mount_units;
use core_ops::core::unit::{apply_service_mount_dependencies, render_mount_unit};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, EnabledState, Invariant, MountDeclaration,
    MountDependency, MountVerificationMode, ObservedState, PathDependencyMode, QuadletType,
    RestartPolicy, UnitDependencyMode, Workload,
};

fn workload(name: &str) -> Workload {
    workload_with_type(name, QuadletType::Container)
}

fn workload_with_type(name: &str, quadlet_type: QuadletType) -> Workload {
    let extension = match quadlet_type {
        QuadletType::Container => "container",
        QuadletType::Socket => "socket",
        QuadletType::SocketDropIn => "socket-dropin",
        QuadletType::ConfigFile => "config",
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

    let alpha_prefix = vec![
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

    let volume_prefix = vec![
        "volume.volume".to_string(),
        "volume.volume".to_string(),
    ];
    let container_prefix = vec![
        "container.container".to_string(),
        "container.container".to_string(),
        "container.container".to_string(),
    ];
    let network_prefix = vec![
        "network.network".to_string(),
        "network.network".to_string(),
        "network.network".to_string(),
    ];
    let socket_prefix = vec![
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
    let observed = observed_state(vec![socket_dropin_workload(
        "alpha.socket.d/10-host.conf",
    )]);

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
        *action == core_ops::core::types::PlanActionType::RestartUnit
            && *target == "alpha.socket"
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
        ownership_scope: vec!["immich".to_string()],
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
    assert!(service_contents.contains("After=var-lib-immich-media.automount var-lib-immich-media.mount"));
}
