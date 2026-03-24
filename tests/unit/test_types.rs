use std::path::PathBuf;

use core_ops::core::types::{
    Boundaries, BoundaryScope, EnabledState, Invariant, MountDeclaration, MountDependency,
    MountVerificationMode, PathDependencyMode, PreparedTargetPath, QuadletType, RestartPolicy,
    UnitDependencyMode, Workload,
};
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
        ownership_scope: vec!["immich".to_string()],
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
        owner: Some("1000".to_string()),
        group: Some("1000".to_string()),
        mode: Some("0755".to_string()),
        service_consumed: true,
    };
    let dependency = MountDependency {
        service_name: "immich".to_string(),
        mount_ids: vec!["immich-media".to_string()],
        consumed_paths: vec!["/var/lib/immich/media".to_string()],
        path_dependency_mode: PathDependencyMode::RequiresMountsFor,
        unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
    };

    assert_eq!(prepared.owner.as_deref(), Some("1000"));
    assert_eq!(dependency.mount_ids, vec!["immich-media"]);
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
