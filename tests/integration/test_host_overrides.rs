use crate::integration::source_repo_support::{
    git_init_commit, load_with_host, materialize_skeleton, write_host_yaml,
};

/// FR-014: base service drop-ins precede host drop-ins lexicographically,
/// mirroring systemd's own override order. Asserts the merged container
/// content carries both the base layer and the host layer in that order,
/// and that socket drop-ins surface as separate workloads with the host
/// drop-in winning over an overlapping base directive.
#[test]
fn applies_host_overrides_after_base_dropins() {
    let (tmp, services, hosts) = materialize_skeleton();

    // Base service: traefik with a container, a socket, and one drop-in
    // on each unit.
    let svc = services.join("traefik");
    std::fs::create_dir_all(svc.join("quadlet/traefik.container.d")).unwrap();
    std::fs::create_dir_all(svc.join("systemd/traefik.socket.d")).unwrap();
    std::fs::write(
        svc.join("quadlet/traefik.container"),
        "[Container]\nImage=docker.io/library/traefik\n",
    )
    .unwrap();
    std::fs::write(
        svc.join("quadlet/traefik.container.d/10-defaults.conf"),
        "[Service]\nEnvironment=TRAEFIK_LOG_LEVEL=INFO\n",
    )
    .unwrap();
    std::fs::write(
        svc.join("systemd/traefik.socket"),
        "[Socket]\nListenStream=127.0.0.1:8080\n",
    )
    .unwrap();
    std::fs::write(
        svc.join("systemd/traefik.socket.d/10-defaults.conf"),
        "[Socket]\nNoDelay=true\n",
    )
    .unwrap();

    // Host overlay: kadath contributes a host drop-in on each unit.
    let overlay = hosts.join("kadath/traefik");
    std::fs::create_dir_all(overlay.join("quadlet/traefik.container.d")).unwrap();
    std::fs::create_dir_all(overlay.join("systemd/traefik.socket.d")).unwrap();
    std::fs::write(
        overlay.join("quadlet/traefik.container.d/20-host.conf"),
        "[Service]\nEnvironment=TRAEFIK_HOST=kadath\n",
    )
    .unwrap();
    std::fs::write(
        overlay.join("systemd/traefik.socket.d/20-host.conf"),
        "[Socket]\nListenStream=127.0.0.1:8081\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "kadath", &["traefik"]);
    let rev = git_init_commit(tmp.path());

    let desired = load_with_host(tmp.path(), &rev, "kadath").expect("load");

    // Container content: base before host (FR-014 order).
    let container = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.container")
        .expect("traefik container workload");
    assert!(
        container.quadlet_contents.contains("TRAEFIK_LOG_LEVEL=INFO"),
        "missing base drop-in content: {}",
        container.quadlet_contents
    );
    assert!(
        container.quadlet_contents.contains("TRAEFIK_HOST=kadath"),
        "missing host drop-in content: {}",
        container.quadlet_contents
    );
    let base_pos = container
        .quadlet_contents
        .find("TRAEFIK_LOG_LEVEL=INFO")
        .unwrap();
    let host_pos = container
        .quadlet_contents
        .find("TRAEFIK_HOST=kadath")
        .unwrap();
    assert!(
        base_pos < host_pos,
        "host drop-in must follow base drop-in"
    );

    // Socket drop-ins surface as separate workloads. Host drop-in wins
    // over the overlapping base ListenStream.
    let socket_base = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.socket.d/10-defaults.conf")
        .expect("base socket drop-in workload");
    assert!(socket_base.quadlet_contents.contains("NoDelay=true"));

    let socket_host = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.socket.d/20-host.conf")
        .expect("host socket drop-in workload");
    assert!(socket_host
        .quadlet_contents
        .contains("ListenStream=127.0.0.1:8081"));
}
