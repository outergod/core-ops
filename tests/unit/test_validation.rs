use core_ops::core::boundaries::enforce_plan_boundaries;
use core_ops::core::types::{
    Boundaries, BoundaryScope, Cause, CauseKind, DependencyEdgeView, DependencyRelation,
    DesiredState, EnabledState, Invariant, ManagedObjectRef, MountDeclaration, MountDependency,
    MountVerificationMode, PathDependencyMode, PlanAction, PlanActionType, PlanEntry,
    PlanEntryAction, PlanOutputView, PlanSummaryView, PreparedTargetPath, QuadletType,
    ReconciliationPlan, RestartPolicy, RevisionContext, UnitDependencyMode, Workload,
};
use core_ops::core::validation::{
    detect_semantic_dependency_cycle, validate_canonical_object_identity, validate_desired_state,
    validate_mount_model, validate_plan_output_view, validate_retry_signature,
    validate_rollback_candidate,
};

fn base_desired() -> DesiredState {
    DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![Workload {
            name: "alpha".to_string(),
            quadlet_type: QuadletType::Container,
            quadlet_contents: "[Container]".to_string(),
            systemd_unit_name: "alpha.container".to_string(),
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
    }
}

#[test]
fn validates_ok_when_invariants_and_boundaries_present() {
    let desired = base_desired();

    assert!(validate_desired_state(&desired).is_ok());
}

#[test]
fn rejects_unsupported_plan_actions() {
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![
            PlanAction {
                action_type: PlanActionType::ReloadSystemd,
                target: "alpha".to_string(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            PlanAction {
                action_type: PlanActionType::WriteQuadlet,
                target: "alpha".to_string(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
        ],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let result = enforce_plan_boundaries(&plan);
    assert!(result.is_ok());
}

#[test]
fn rejects_unknown_plan_action_types() {
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![PlanAction {
            action_type: PlanActionType::Unknown("mystery".to_string()),
            target: "alpha".to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };

    let result = enforce_plan_boundaries(&plan);
    assert!(result.is_err());
}

#[test]
fn fails_when_missing_invariant() {
    let mut desired = base_desired();
    desired.invariants = vec![Invariant::BoundariesDeclared];

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("DeterministicPlan"));
}

#[test]
fn fails_when_missing_boundary_scope() {
    let mut desired = base_desired();
    desired.boundaries.scopes.clear();

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("QuadletSystemd"));
}

#[test]
fn allows_same_name_for_distinct_unit_names() {
    let mut desired = base_desired();
    let mut extra = desired.workloads[0].clone();
    extra.quadlet_type = QuadletType::Socket;
    extra.systemd_unit_name = "alpha.socket".to_string();
    desired.workloads.push(extra);

    assert!(validate_desired_state(&desired).is_ok());
}

#[test]
fn fails_on_duplicate_unit_name() {
    let mut desired = base_desired();
    let mut extra = desired.workloads[0].clone();
    extra.name = "beta".to_string();
    desired.workloads.push(extra);

    let err = validate_desired_state(&desired).unwrap_err();
    assert!(err.message.contains("duplicate unit"));
}

#[test]
fn rejects_automount_for_non_network_mounts() {
    let mounts = vec![MountDeclaration {
        id: "local-data".to_string(),
        target_path: "/srv/data".to_string(),
        source: "/dev/vdb1".to_string(),
        fstype: "xfs".to_string(),
        mount_options: Vec::new(),
        network_backed: false,
        automount: true,
        verification_mode: MountVerificationMode::UnitAndPath,
        prepared_path: None,
    }];

    let err = validate_mount_model(&mounts, &[], Some(&["alpha".to_string()])).unwrap_err();
    assert!(err.message.contains("network-backed"));
}

#[test]
fn rejects_prepared_path_that_differs_from_mount_target() {
    let mounts = vec![MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/var/lib/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: Vec::new(),
        network_backed: true,
        automount: false,
        verification_mode: MountVerificationMode::UnitAndPath,
        prepared_path: Some(PreparedTargetPath {
            path: "/srv/immich/media".to_string(),
            create_if_missing: true,
        }),
    }];

    let err = validate_mount_model(&mounts, &[], Some(&["immich".to_string()])).unwrap_err();
    assert!(err.message.contains("must match mount target"));
}

#[test]
fn rejects_mount_dependency_for_unknown_mount_reference() {
    let mounts = vec![MountDeclaration {
        id: "immich-media".to_string(),
        target_path: "/var/lib/immich/media".to_string(),
        source: "nas:/media".to_string(),
        fstype: "nfs".to_string(),
        mount_options: Vec::new(),
        network_backed: true,
        automount: false,
        verification_mode: MountVerificationMode::UnitAndPath,
        prepared_path: None,
    }];
    let dependencies = vec![MountDependency {
        service_name: "immich".to_string(),
        mount_ids: vec!["unknown".to_string()],
        consumed_paths: vec!["/var/lib/immich/media".to_string()],
        path_dependency_mode: PathDependencyMode::RequiresMountsFor,
        unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
    }];

    let err =
        validate_mount_model(&mounts, &dependencies, Some(&["immich".to_string()])).unwrap_err();
    assert!(err.message.contains("unknown mount declaration"));
}

#[test]
fn canonical_object_identity_rejects_whitespace() {
    let err = validate_canonical_object_identity("alpha service").unwrap_err();
    assert!(err.message.contains("whitespace"));
}

#[test]
fn semantic_dependency_cycle_is_detected() {
    let err = detect_semantic_dependency_cycle(&[
        ("a".to_string(), "b".to_string()),
        ("b".to_string(), "a".to_string()),
    ])
    .unwrap_err();
    assert!(err.message.contains("cycle"));
}

#[test]
fn rollback_candidate_must_be_retained_and_scope_compatible() {
    let err = validate_rollback_candidate(
        &["rev-1".to_string()],
        "host:alpha",
        "rev-1",
        "host:beta",
        true,
    )
    .unwrap_err();
    assert!(err.message.contains("scope mismatch"));

    let err = validate_rollback_candidate(
        &["rev-1".to_string()],
        "host:alpha",
        "rev-2",
        "host:alpha",
        false,
    )
    .unwrap_err();
    assert!(err.message.contains("not retained"));
}

#[test]
fn retry_signature_requires_object_set_and_pattern() {
    let err = validate_retry_signature("missing-parts").unwrap_err();
    assert!(err.message.contains("invalid retry signature"));
    assert!(validate_retry_signature("alpha|timeout").is_ok());
}

#[test]
fn plan_output_validation_rejects_non_sequential_order_indices() {
    let err = validate_plan_output_view(&PlanOutputView {
        view_kind: "plan".to_string(),
        revision_context: RevisionContext {
            target_revision: "rev-2".to_string(),
            requested_repository: None,
            requested_ref: None,
            last_applied_requested_repository: None,
            last_applied_requested_ref: None,
            scope_id: None,
            last_applied_revision: None,
            change_revision: None,
        },
        summary: PlanSummaryView {
            changed_count: 1,
            unchanged_count: 0,
            blocked_count: 0,
            skipped_count: 0,
            total_count: Some(1),
        },
        entries: vec![PlanEntry {
            object: ManagedObjectRef {
                resource_type: "service".to_string(),
                name: "alpha.service".to_string(),
                display_id: "service/alpha.service".to_string(),
            },
            action: PlanEntryAction::Update,
            causes: vec![Cause {
                kind: CauseKind::DesiredChange,
                summary: "image changed".to_string(),
                source_object: None,
                details: None,
            }],
            dependencies: vec![DependencyEdgeView {
                relation: DependencyRelation::Prerequisite,
                object: ManagedObjectRef {
                    resource_type: "config".to_string(),
                    name: "config:/etc/alpha/env".to_string(),
                    display_id: "config/etc/alpha/env".to_string(),
                },
            }],
            order_index: 1,
            diff: None,
            unchanged: Some(false),
            notes: None,
        }],
    })
    .unwrap_err();

    assert!(err.message.contains("sequential order indices"));
}

#[test]
fn plan_output_validation_rejects_non_noop_entries_without_causes() {
    let err = validate_plan_output_view(&PlanOutputView {
        view_kind: "plan".to_string(),
        revision_context: RevisionContext {
            target_revision: "rev-2".to_string(),
            requested_repository: None,
            requested_ref: None,
            last_applied_requested_repository: None,
            last_applied_requested_ref: None,
            scope_id: None,
            last_applied_revision: None,
            change_revision: None,
        },
        summary: PlanSummaryView {
            changed_count: 1,
            unchanged_count: 0,
            blocked_count: 0,
            skipped_count: 0,
            total_count: Some(1),
        },
        entries: vec![PlanEntry {
            object: ManagedObjectRef {
                resource_type: "service".to_string(),
                name: "alpha.service".to_string(),
                display_id: "service/alpha.service".to_string(),
            },
            action: PlanEntryAction::Update,
            causes: Vec::new(),
            dependencies: Vec::new(),
            order_index: 0,
            diff: None,
            unchanged: Some(false),
            notes: None,
        }],
    })
    .unwrap_err();

    assert!(err.message.contains("must include a cause"));
}
