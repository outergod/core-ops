use std::path::PathBuf;

use core_ops::core::types::{
    Boundaries, BoundaryScope, Cause, CauseKind, ConvergenceStatus, DependencyEdgeKind,
    DependencyEdgeView, DependencyRelation, DeterministicPersistedState, EnabledState,
    ExplainOutputView, Invariant, ManagedObjectKind, ManagedObjectRef, MountDeclaration,
    MountDependency, MountVerificationMode, NormalizedManagedObject, NormalizedSnapshot,
    PathDependencyMode, PlanEntry, PlanEntryAction, PlanOutputView, PlanSummaryView,
    PreparedTargetPath, QuadletType, RestartPolicy, RevisionContext, RollbackEligibility,
    RollbackTargetCandidate, RuntimeVerificationSignal, SemanticDependencyEdge,
    SemanticDependencyGraph, SemanticDependencyNode, SemanticDiffKind, SemanticDiffView,
    UnitDependencyMode, Workload,
};
use core_ops::core::unit::systemd_unit_for_quadlet_file;
use core_ops::io::quadlet::read_quadlet_dir;

#[test]
fn boundaries_reports_scopes() {
    let boundaries = Boundaries {
        scopes: vec![BoundaryScope::QuadletSystemd],
    };

    assert!(boundaries.has_scope(BoundaryScope::QuadletSystemd));
}

#[test]
fn workload_key_is_name() {
    let workload = Workload {
        name: "alpha".to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: "[Container]".to_string(),
        systemd_unit_name: "alpha.container".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };

    assert_eq!(workload.key(), "alpha");
}

#[test]
fn invariants_can_be_listed_explicitly() {
    let invariants = vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan];

    assert!(invariants.contains(&Invariant::BoundariesDeclared));
    assert!(invariants.contains(&Invariant::DeterministicPlan));
}

#[test]
fn quadlet_type_parsing_supports_socket_and_volume() {
    let dir = temp_dir("core_ops_unit_quadlets");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    std::fs::write(dir.join("alpha.container"), "[Container]").expect("write container");
    std::fs::write(dir.join("beta.socket"), "[Socket]").expect("write socket");
    std::fs::write(dir.join("gamma.volume"), "[Volume]").expect("write volume");

    let mut workloads = read_quadlet_dir(&dir).expect("read quadlet dir");
    workloads.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(workloads.len(), 3);
    assert_eq!(workloads[0].quadlet_type, QuadletType::Container);
    assert_eq!(workloads[1].quadlet_type, QuadletType::Socket);
    assert_eq!(workloads[2].quadlet_type, QuadletType::Volume);
}

#[test]
fn mount_declaration_derives_native_unit_names() {
    let mount = MountDeclaration {
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

    assert_eq!(mount.mount_unit_name(), "var-lib-immich-media.mount");
    assert_eq!(
        mount.automount_unit_name().as_deref(),
        Some("var-lib-immich-media.automount")
    );
}

#[test]
fn prepared_target_metadata_and_dependency_identity_are_explicit() {
    let prepared = PreparedTargetPath {
        path: "/var/lib/immich/media".to_string(),
        create_if_missing: true,
    };
    let dependency = MountDependency {
        service_name: "immich".to_string(),
        mount_ids: vec!["immich-media".to_string()],
        consumed_paths: vec!["/var/lib/immich/media".to_string()],
        path_dependency_mode: PathDependencyMode::RequiresMountsFor,
        unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
    };

    assert!(prepared.create_if_missing);
    assert_eq!(dependency.mount_ids, vec!["immich-media"]);
}

#[test]
fn quadlet_runtime_unit_names_follow_quadlet_rules() {
    assert_eq!(
        systemd_unit_for_quadlet_file("alpha.container"),
        "alpha.service"
    );
    assert_eq!(systemd_unit_for_quadlet_file("beta.socket"), "beta.socket");
    assert_eq!(
        systemd_unit_for_quadlet_file("gamma.volume"),
        "gamma-volume.service"
    );
    assert_eq!(
        systemd_unit_for_quadlet_file("immich.network"),
        "immich-network.service"
    );
    assert_eq!(systemd_unit_for_quadlet_file("pod.pod"), "pod-pod.service");
}

#[test]
fn deterministic_reconciliation_types_are_explicit() {
    let snapshot = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![NormalizedManagedObject {
            object_id: "alpha.container".to_string(),
            object_kind: ManagedObjectKind::QuadletResource,
            material_fields: Default::default(),
            dependency_refs: vec!["config:/etc/alpha/env".to_string()],
        }],
    };
    let graph = SemanticDependencyGraph {
        nodes: vec![SemanticDependencyNode {
            object_id: "alpha.container".to_string(),
            object_kind: ManagedObjectKind::QuadletResource,
            ordering_key: "alpha.container".to_string(),
        }],
        edges: vec![SemanticDependencyEdge {
            from_object_id: "config:/etc/alpha/env".to_string(),
            to_object_id: "alpha.container".to_string(),
            edge_kind: DependencyEdgeKind::Explicit,
            reason: "rendered config precedes container".to_string(),
        }],
    };
    let rollback = RollbackTargetCandidate {
        target_revision_id: "rev-0".to_string(),
        scope_id: snapshot.scope_id.clone(),
        eligibility: RollbackEligibility::Eligible,
        reason: "retained successful snapshot".to_string(),
    };
    let signal = RuntimeVerificationSignal {
        object_id: "alpha.container".to_string(),
        unit_name: Some("alpha.service".to_string()),
        active_state: Some("active".to_string()),
        details: None,
    };
    let persisted = DeterministicPersistedState {
        schema_version: 1,
        current_scope: snapshot.scope_id.clone(),
        retained_snapshots: Vec::new(),
        latest_convergence: None,
        latest_rollback_target: Some(rollback),
    };

    assert_eq!(snapshot.objects.len(), 1);
    assert_eq!(graph.edges[0].edge_kind, DependencyEdgeKind::Explicit);
    assert_eq!(signal.active_state.as_deref(), Some("active"));
    assert_eq!(persisted.current_scope, "host:alpha");
}

#[test]
fn convergence_status_covers_foundational_outcomes() {
    let statuses = [
        ConvergenceStatus::Success,
        ConvergenceStatus::Partial,
        ConvergenceStatus::Blocked,
        ConvergenceStatus::RepeatedFailure,
        ConvergenceStatus::Oscillation,
        ConvergenceStatus::Failed,
    ];

    assert_eq!(statuses.len(), 6);
}

#[test]
fn public_output_types_preserve_stable_identity_and_schema_shape() {
    let object = ManagedObjectRef {
        resource_type: "service".to_string(),
        name: "alpha.service".to_string(),
        display_id: "service/alpha.service".to_string(),
    };
    let entry = PlanEntry {
        object: object.clone(),
        action: PlanEntryAction::Update,
        causes: vec![Cause {
            kind: CauseKind::DesiredChange,
            summary: "service definition changed".to_string(),
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
        order_index: 0,
        diff: Some(SemanticDiffView {
            kind: SemanticDiffKind::SemanticOnly,
            summary: "image field changed".to_string(),
            unified_diff: None,
            details: None,
        }),
        unchanged: Some(false),
        notes: None,
    };
    let plan = PlanOutputView {
        view_kind: "plan".to_string(),
        revision_context: RevisionContext {
            target_revision: "rev-2".to_string(),
            requested_repository: None,
            requested_ref: None,
            last_applied_requested_repository: None,
            last_applied_requested_ref: None,
            scope_id: None,
            last_applied_revision: Some("rev-1".to_string()),
            change_revision: Some("rev-2".to_string()),
        },
        summary: PlanSummaryView {
            changed_count: 1,
            unchanged_count: 0,
            blocked_count: 0,
            skipped_count: 0,
            total_count: Some(1),
        },
        entries: vec![entry],
    };
    let explain = ExplainOutputView {
        view_kind: "explain".to_string(),
        revision_context: plan.revision_context.clone(),
        object,
        action_or_outcome: "update".to_string(),
        causes: plan.entries[0].causes.clone(),
        dependencies: plan.entries[0].dependencies.clone(),
        dependency_context: None,
        diff: plan.entries[0].diff.clone(),
        metadata: None,
        x_coreops: None,
        apply_intent: None,
        summary: None,
        history: None,
    };

    let plan_json = serde_json::to_value(&plan).expect("serialize plan output");
    let explain_json = serde_json::to_value(&explain).expect("serialize explain output");

    assert_eq!(plan_json["view_kind"].as_str(), Some("plan"));
    assert_eq!(
        plan_json["entries"][0]["object"]["display_id"].as_str(),
        Some("service/alpha.service")
    );
    assert_eq!(
        explain_json["dependencies"][0]["relation"].as_str(),
        Some("prerequisite")
    );
}

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("{prefix}_{stamp}"));
    path
}
