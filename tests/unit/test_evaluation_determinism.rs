use core_ops::core::evaluate::evaluate_desired_state;
use core_ops::core::reconcile::reconcile_apply;
use core_ops::core::types::{
    ArtifactSource, Boundaries, BoundaryScope, DesiredState, DropInSource, EnabledState,
    EvaluationInput, HostDeclaration, HostOverlaySet, Invariant, MountDeclaration,
    MountDependency, MountVerificationMode, ObservedState, ObservedUnit, PathDependencyMode,
    QuadletType, RestartPolicy, ServiceCatalog, ServiceDefinition, UnitActiveState,
    UnitDependencyMode, Workload,
};
use core_ops::core::reconcile::ReconcileDependencies;
use std::collections::BTreeMap;

#[test]
fn evaluation_is_deterministic_for_same_input() {
    let service = ServiceDefinition {
        name: "alpha".to_string(),
        artifacts: vec![
            ArtifactSource {
                name: "beta.container".to_string(),
                quadlet_type: QuadletType::Container,
                contents: "B".to_string(),
                source_path: "/services/alpha/beta.container".to_string(),
            },
            ArtifactSource {
                name: "alpha.container".to_string(),
                quadlet_type: QuadletType::Container,
                contents: "A".to_string(),
                source_path: "/services/alpha/alpha.container".to_string(),
            },
        ],
        base_dropins: vec![DropInSource {
            target: "alpha.container".to_string(),
            contents: "X".to_string(),
            source_path: "/services/alpha/alpha.container.d/10-x.conf".to_string(),
        }],
        config_files: Vec::new(),
        mount_declarations: Vec::new(),
        service_mounts: Vec::new(),
    };

    let mut services = BTreeMap::new();
    services.insert("alpha".to_string(), service);

    let input = EvaluationInput {
        host: HostDeclaration {
            host: "ulthar".to_string(),
            services: vec!["alpha".to_string()],
        },
        catalog: ServiceCatalog { services },
        overlays: HostOverlaySet {
            host: "ulthar".to_string(),
            overrides: vec![DropInSource {
                target: "alpha.container".to_string(),
                contents: "Y".to_string(),
                source_path: "/hosts/ulthar/overrides/alpha.container.d/20-y.conf".to_string(),
            }],
            config_overrides: Vec::new(),
            mount_overrides: Vec::new(),
            service_mount_overrides: BTreeMap::new(),
        },
    };

    let first = evaluate_desired_state(&input).expect("evaluate");
    let second = evaluate_desired_state(&input).expect("evaluate");

    assert_eq!(first, second);
    assert_eq!(first.artifacts[0].name, "alpha.container");
    assert_eq!(first.artifacts[1].name, "beta.container");
}

#[test]
fn degraded_mount_failure_is_deterministic() {
    let desired = DesiredState {
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
            mount_options: Vec::new(),
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
    };
    let observed = ObservedState {
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
    let deps = ReconcileDependencies {
        load_desired: &|| Ok(desired.clone()),
        read_observed: &|_| Ok(observed.clone()),
        apply_plan: &|_, _| Ok(()),
    };

    let first = reconcile_apply(&deps).expect("reconcile");
    let second = reconcile_apply(&deps).expect("reconcile");

    assert_eq!(first.run.summary, "mount degraded");
    assert_eq!(first.run.summary, second.run.summary);
}
