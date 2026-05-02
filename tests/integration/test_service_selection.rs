use crate::integration::source_repo_support::{
    git_init_commit, load_with_host, materialize_skeleton, write_host_yaml,
};

/// Asserts host.yaml-driven service selection: each host's workload set
/// is exactly the union of its declared services. Subsumes the
/// fixture-backed multi-host case removed in T118.
#[test]
fn selects_services_per_host() {
    let (tmp, services, hosts) = materialize_skeleton();

    for svc in ["traefik", "immich", "vector"] {
        let dir = services.join(svc);
        std::fs::create_dir_all(dir.join("quadlet")).unwrap();
        std::fs::write(
            dir.join(format!("quadlet/{svc}.container")),
            "[Container]\nImage=docker.io/library/alpine\n",
        )
        .unwrap();
    }
    // immich gets a sibling volume to confirm multi-artifact services
    // surface every artifact when selected.
    std::fs::write(
        services.join("immich/quadlet/immich.volume"),
        "[Volume]\n",
    )
    .unwrap();

    write_host_yaml(&hosts, "kadath", &["traefik", "immich"]);
    write_host_yaml(&hosts, "rlyeh", &["traefik", "vector"]);
    let rev = git_init_commit(tmp.path());

    let kadath = load_with_host(tmp.path(), &rev, "kadath").expect("load kadath");
    let kadath_units: Vec<&str> = kadath
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.as_str())
        .collect();
    assert!(kadath_units.contains(&"traefik.container"));
    assert!(kadath_units.contains(&"immich.container"));
    assert!(kadath_units.contains(&"immich.volume"));
    assert!(!kadath_units.contains(&"vector.container"));

    let rlyeh = load_with_host(tmp.path(), &rev, "rlyeh").expect("load rlyeh");
    let rlyeh_units: Vec<&str> = rlyeh
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.as_str())
        .collect();
    assert!(rlyeh_units.contains(&"traefik.container"));
    assert!(rlyeh_units.contains(&"vector.container"));
    assert!(!rlyeh_units.contains(&"immich.container"));
    assert!(!rlyeh_units.contains(&"immich.volume"));
}
