use core_ops::cli::report::build_apply_output;
use core_ops::core::reconcile::normalize_verification_results_for_desired;
use core_ops::core::retry::build_retry_observation;
use core_ops::core::types::{
    Boundaries, BoundaryScope, ConvergenceStatus, DependencyEdgeKind, DesiredState,
    DeterministicActionClass, DeterministicConvergenceRecord, DeterministicPlannedAction,
    DeterministicReconciliationPlan, EnabledState, HostInfo, Invariant, ManagedObjectKind,
    MountDeclaration, MountVerificationMode, ObservedState, ObservedUnit, PathDependencyMode,
    QuadletType, RestartPolicy, SemanticDependencyEdge, SemanticDependencyGraph,
    SemanticDependencyNode, ServiceDependencyEdit, UnitActiveState, UnitDependencyMode,
    VerificationResult, VerificationStatus, Workload,
};
use core_ops::core::unit::{
    apply_service_mount_dependencies, render_automount_unit, render_mount_unit,
};
use core_ops::core::verify::{evaluate_convergence, verify_state};

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

fn observed_state(units: Vec<ObservedUnit>) -> ObservedState {
    ObservedState {
        observed_revision_id: None,
        units,
        workloads: Vec::new(),
        last_reconcile_id: None,
        host_info: Some(HostInfo {
            hostname: "alpha".to_string(),
            os_id: "fedora".to_string(),
        }),
    }
}

#[test]
fn verify_container_requires_active_unit() {
    let desired = desired_state(vec![workload(
        "alpha",
        QuadletType::Container,
        "alpha.container",
    )]);
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
fn verification_results_normalize_runtime_units_to_managed_object_ids() {
    let desired = desired_state(vec![workload(
        "frontend",
        QuadletType::Container,
        "frontend.container",
    )]);
    let normalized = normalize_verification_results_for_desired(
        &desired,
        vec![VerificationResult {
            target: "frontend.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("unit not active: failed".to_string()),
        }],
    );

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].target, "frontend.container");
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
        requested_repository: None,
        requested_ref: None,
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
        requested_repository: None,
        requested_ref: None,
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
        prepared_path: None,
    };
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
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
    assert!(results
        .iter()
        .all(|result| result.status == VerificationStatus::Success));
}

#[test]
fn verify_automount_backed_mount_accepts_missing_mount_unit_when_automount_is_active() {
    let mount = MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/srv/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: vec!["rw".to_string()],
        network_backed: true,
        automount: true,
        verification_mode: MountVerificationMode::UnitAndPath,
        prepared_path: None,
    };
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
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
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "srv-immich-media.automount".to_string(),
        active_state: UnitActiveState::Active,
        enabled_state: EnabledState::Enabled,
    }]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 2);
    assert!(results
        .iter()
        .all(|result| result.status == VerificationStatus::Success));
}

#[test]
fn apply_output_events_follow_phase_and_plan_order() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![
            DeterministicPlannedAction {
                object_id: "alpha.service".to_string(),
                classification: DeterministicActionClass::Update,
                reason: "drift".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: [(
                    "contents".to_string(),
                    "desired=a actual=b applied=a".to_string(),
                )]
                .into_iter()
                .collect(),
            },
            DeterministicPlannedAction {
                object_id: "beta.service".to_string(),
                classification: DeterministicActionClass::Restart,
                reason: "dependency changed".to_string(),
                dependency_context: vec!["alpha.service".to_string()],
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "gamma.service".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "alpha.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "alpha.service".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "beta.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "beta.service".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "gamma.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "gamma.service".to_string(),
                },
            ],
            edges: vec![SemanticDependencyEdge {
                from_object_id: "alpha.service".to_string(),
                to_object_id: "beta.service".to_string(),
                edge_kind: DependencyEdgeKind::Explicit,
                reason: "prerequisite".to_string(),
            }],
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec![
            "alpha.service".to_string(),
            "beta.service".to_string(),
            "gamma.service".to_string(),
        ],
        completed_actions: vec!["alpha.service".to_string(), "beta.service".to_string()],
        failed_actions: Vec::new(),
        can_continue: false,
    };
    let output = build_apply_output(&plan, &[], Some(&convergence));

    for (index, phase) in output.phases.iter().enumerate() {
        assert_eq!(phase.sequence, index);
    }
    for (index, event) in output.events.iter().enumerate() {
        assert_eq!(event.sequence, output.phases.len() + index);
    }

    let object_order = output
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.state,
                core_ops::core::types::ExecutionState::Pending
                    | core_ops::core::types::ExecutionState::Unchanged
            )
        })
        .map(|event| event.object.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        object_order,
        vec!["alpha.service", "beta.service", "gamma.service"]
    );
}

#[test]
fn apply_output_uses_recovered_terminal_vocabulary_for_recovery_actions() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "alpha.service".to_string(),
            classification: DeterministicActionClass::Recover,
            reason: "runtime reconciliation required: unit not active: failed".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "alpha.service".to_string(),
                object_kind: ManagedObjectKind::GeneratedUnit,
                ordering_key: "alpha.service".to_string(),
            }],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["alpha.service".to_string()],
        completed_actions: vec!["alpha.service".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let output = build_apply_output(&plan, &[], Some(&convergence));
    let terminal = output
        .events
        .iter()
        .find(|event| {
            matches!(
                event.event_kind,
                core_ops::core::types::ExecutionEventKind::ObjectTerminal
            )
        })
        .expect("terminal event");

    assert_eq!(terminal.object.name, "alpha.service");
    assert_eq!(
        terminal.state,
        core_ops::core::types::ExecutionState::Recovered
    );
    assert_eq!(
        terminal.action,
        Some(core_ops::core::types::PlanEntryAction::Recover)
    );
}

struct MountInfoGuard;

impl Drop for MountInfoGuard {
    fn drop(&mut self) {
        std::env::remove_var("CORE_OPS_MOUNTINFO_PATH");
    }
}

#[test]
fn convergence_status_classifies_repeated_failure_and_honors_retry_budget() {
    let desired = desired_state(vec![workload(
        "alpha",
        QuadletType::Container,
        "alpha.container",
    )]);
    let observed = observed_state(vec![ObservedUnit {
        unit_name: "alpha.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Enabled,
    }]);
    let verification_results = verify_state(&desired, &observed);
    let history = vec![
        build_retry_observation(1, &verification_results),
        build_retry_observation(2, &verification_results),
        build_retry_observation(3, &verification_results),
    ];

    let convergence = evaluate_convergence(&desired, &observed, &history, 3);

    assert_eq!(convergence.status, ConvergenceStatus::RepeatedFailure);
    assert_eq!(convergence.attempt_count, 3);
    assert!(!convergence.can_continue);
}

#[test]
fn convergence_status_classifies_blocked_and_success_cases() {
    let desired = desired_state(vec![workload(
        "alpha",
        QuadletType::Container,
        "alpha.container",
    )]);
    let blocked_observed = observed_state(vec![ObservedUnit {
        unit_name: "alpha.service".to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Enabled,
    }]);
    let blocked_history = vec![build_retry_observation(
        1,
        &[VerificationResult {
            target: "alpha.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("blocked: unit not active".to_string()),
        }],
    )];
    let blocked = evaluate_convergence(&desired, &blocked_observed, &blocked_history, 3);
    assert_eq!(blocked.status, ConvergenceStatus::Blocked);

    let success_observed = observed_state(vec![ObservedUnit {
        unit_name: "alpha.service".to_string(),
        active_state: UnitActiveState::Active,
        enabled_state: EnabledState::Enabled,
    }]);
    let success_history = vec![build_retry_observation(
        1,
        &verify_state(&desired, &success_observed),
    )];
    let success = evaluate_convergence(&desired, &success_observed, &success_history, 3);
    assert_eq!(success.status, ConvergenceStatus::Success);
}

fn socket_workload(unit_name: &str, target_service: &str) -> Workload {
    let contents = format!(
        "[Socket]\nListenStream=80\nFileDescriptorName=web\nService={target_service}\n\n[Install]\nWantedBy=sockets.target\n"
    );
    Workload {
        name: unit_name.trim_end_matches(".socket").to_string(),
        quadlet_type: QuadletType::Socket,
        quadlet_contents: contents,
        systemd_unit_name: unit_name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn config_workload(target_path: &str) -> Workload {
    Workload {
        name: target_path.to_string(),
        quadlet_type: QuadletType::ConfigFile,
        quadlet_contents: "# config".to_string(),
        systemd_unit_name: target_path.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

#[test]
fn verify_socket_activated_service_inactive_passes_when_socket_is_active() {
    // Regression for fix-socket-activated-verification: a socket-activated
    // service is correctly Inactive until first connection. Verification must
    // accept that state when at least one of its triggering sockets is Active,
    // because systemd will start the service on demand.
    let desired = desired_state(vec![
        workload("traefik", QuadletType::Container, "traefik.container"),
        socket_workload("http.socket", "traefik.service"),
        socket_workload("https.socket", "traefik.service"),
    ]);
    let observed = observed_state(vec![
        ObservedUnit {
            unit_name: "traefik.service".to_string(),
            active_state: UnitActiveState::Inactive,
            enabled_state: EnabledState::Enabled,
        },
        ObservedUnit {
            unit_name: "http.socket".to_string(),
            active_state: UnitActiveState::Active,
            enabled_state: EnabledState::Enabled,
        },
        ObservedUnit {
            unit_name: "https.socket".to_string(),
            active_state: UnitActiveState::Active,
            enabled_state: EnabledState::Enabled,
        },
    ]);

    let results = verify_state(&desired, &observed);
    assert_eq!(results.len(), 3, "results: {:?}", results);
    assert!(
        results
            .iter()
            .all(|r| r.status == VerificationStatus::Success),
        "every workload should verify; got {:?}",
        results
    );
}

#[test]
fn verify_socket_activated_service_failed_state_still_fails() {
    // The Failed state means the service started and crashed. Active sockets
    // do not absolve a Failed service, only an Inactive one.
    let desired = desired_state(vec![
        workload("traefik", QuadletType::Container, "traefik.container"),
        socket_workload("http.socket", "traefik.service"),
    ]);
    let observed = observed_state(vec![
        ObservedUnit {
            unit_name: "traefik.service".to_string(),
            active_state: UnitActiveState::Failed,
            enabled_state: EnabledState::Enabled,
        },
        ObservedUnit {
            unit_name: "http.socket".to_string(),
            active_state: UnitActiveState::Active,
            enabled_state: EnabledState::Enabled,
        },
    ]);

    let results = verify_state(&desired, &observed);
    let service_result = results
        .iter()
        .find(|r| r.target == "traefik.service")
        .expect("traefik.service result");
    assert_eq!(service_result.status, VerificationStatus::Failure);
}

#[test]
fn verify_socket_activated_service_inactive_socket_inactive_fails() {
    // No Active socket -> nothing will trigger the service -> genuine failure.
    let desired = desired_state(vec![
        workload("traefik", QuadletType::Container, "traefik.container"),
        socket_workload("http.socket", "traefik.service"),
    ]);
    let observed = observed_state(vec![
        ObservedUnit {
            unit_name: "traefik.service".to_string(),
            active_state: UnitActiveState::Inactive,
            enabled_state: EnabledState::Enabled,
        },
        ObservedUnit {
            unit_name: "http.socket".to_string(),
            active_state: UnitActiveState::Inactive,
            enabled_state: EnabledState::Enabled,
        },
    ]);

    let results = verify_state(&desired, &observed);
    let service_result = results
        .iter()
        .find(|r| r.target == "traefik.service")
        .expect("traefik.service result");
    assert_eq!(service_result.status, VerificationStatus::Failure);
}

#[test]
fn alias_normalisation_does_not_misroute_failures_to_config_files() {
    // Regression for fix-socket-activated-verification, alias half: a config
    // file's target_path passed through systemd_unit_for_quadlet_file's
    // catch-all yields `<stem>.service`, which used to collide with the real
    // service's runtime unit name and steal failure attribution. The fix is
    // to skip ConfigFile workloads when populating desired_target_aliases.
    let desired = desired_state(vec![
        workload("traefik", QuadletType::Container, "traefik.container"),
        config_workload("/etc/traefik/traefik.toml"),
    ]);

    let normalized = normalize_verification_results_for_desired(
        &desired,
        vec![VerificationResult {
            target: "traefik.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("unit not active: Inactive".to_string()),
        }],
    );

    assert_eq!(normalized.len(), 1);
    assert_eq!(
        normalized[0].target, "traefik.container",
        "traefik.service failure must route to traefik.container, not the config file"
    );
}

#[test]
fn alias_normalisation_does_not_misroute_failures_to_socket_dropins() {
    // Same shape as the ConfigFile case but for SocketDropIn. A drop-in's
    // systemd_unit_name is `<unit>.socket.d/<file>.conf`; passing that through
    // systemd_unit_for_quadlet_file yields a synthesised `.service` that can
    // collide with a real runtime unit. SocketDropIn workloads must not
    // contribute aliases.
    let mut dropin = workload(
        "traefik.socket.d/10-extra.conf",
        QuadletType::SocketDropIn,
        "traefik.socket.d/10-extra.conf",
    );
    dropin.quadlet_contents = "[Socket]\nFileDescriptorName=extra\n".to_string();

    let desired = desired_state(vec![
        workload("traefik", QuadletType::Container, "traefik.container"),
        dropin,
    ]);

    let normalized = normalize_verification_results_for_desired(
        &desired,
        vec![VerificationResult {
            target: "traefik.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("unit not active: Inactive".to_string()),
        }],
    );

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].target, "traefik.container");
}
