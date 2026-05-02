use std::path::Path;
use std::process::Command;

use crate::integration::source_repo_support::{
    git_init_commit, load_source_with_host, load_with_host, materialize_skeleton, write_host_yaml,
};

/// Build a minimal services/<svc>/quadlet/<svc>.container fixture and
/// commit it. Used by the basic loader tests below.
fn init_alpha_repo(repo: &Path) -> String {
    let services = repo.join("services/alpha/quadlet");
    let hosts = repo.join("hosts/example-host");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::create_dir_all(&hosts).expect("hosts");
    std::fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write quadlet");
    std::fs::write(
        hosts.join("host.yaml"),
        "host: example-host\nservices:\n  - alpha\n",
    )
    .expect("write host.yaml");
    git_init_commit(repo)
}

#[test]
fn loads_desired_state_from_quadlet_dir() {
    let (tmp, _services, _hosts) = materialize_skeleton();
    // materialize_skeleton seeded services/ and hosts/ as empty dirs; rewrite
    // them with the alpha fixture under the canonical layout.
    let rev = init_alpha_repo(tmp.path());

    let desired = load_with_host(tmp.path(), &rev, "example-host").expect("load desired");

    assert_eq!(desired.revision_id, rev);
    let alpha = desired
        .workloads
        .iter()
        .find(|w| w.name == "alpha")
        .expect("alpha workload");
    assert_eq!(alpha.systemd_unit_name, "alpha.container");
}

#[test]
fn loads_desired_state_from_git_url_fixture() {
    let (tmp, _services, _hosts) = materialize_skeleton();
    let rev = init_alpha_repo(tmp.path());
    let repo_url = format!("file://{}", tmp.path().display());

    let desired =
        load_source_with_host(&repo_url, &rev, "example-host").expect("load desired");

    assert!(!desired.workloads.is_empty());
}

#[test]
fn tolerates_dotfiles_in_service_directory() {
    // FR-009 reserves `_` and `.` as identifier prefixes; the parser is
    // explicitly tolerant of dotfile metadata under services/<svc>/ and
    // services/<svc>/quadlet/ (.gitkeep, .DS_Store, etc.). Confirm a
    // dotfile next to a real container does not change the workload set.
    let (tmp, services, hosts) = materialize_skeleton();
    let svc_dir = services.join("alpha");
    let quadlet_dir = svc_dir.join("quadlet");
    std::fs::create_dir_all(&quadlet_dir).expect("create quadlet");
    std::fs::write(
        quadlet_dir.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write container");
    std::fs::write(svc_dir.join(".gitkeep"), "").expect("write dotfile at svc root");
    std::fs::write(quadlet_dir.join(".DS_Store"), "").expect("write dotfile in payload");
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let desired = load_with_host(tmp.path(), &rev, "example-host").expect("load desired");

    let alpha_units: Vec<&str> = desired
        .workloads
        .iter()
        .filter(|w| w.name == "alpha")
        .map(|w| w.systemd_unit_name.as_str())
        .collect();
    assert_eq!(alpha_units, vec!["alpha.container"]);
}

#[test]
fn layered_repo_preserves_requested_repository_and_ref() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc_quadlet = services.join("demo/quadlet");
    std::fs::create_dir_all(&svc_quadlet).expect("create services");
    std::fs::write(
        svc_quadlet.join("whoami.container"),
        "[Container]\nImage=quay.io/podman/hello\n",
    )
    .expect("write service quadlet");
    write_host_yaml(&hosts, "uat", &["demo"]);
    let rev = git_init_commit(tmp.path());
    Command::new("git")
        .arg("-C")
        .arg(tmp.path())
        .arg("branch")
        .arg("demo-uat-v2")
        .output()
        .expect("git branch");

    let desired =
        load_with_host(tmp.path(), "demo-uat-v2", "uat").expect("load desired");

    assert_eq!(desired.revision_id, rev);
    assert_eq!(desired.requested_repository.as_deref(), tmp.path().to_str());
    assert_eq!(desired.requested_ref.as_deref(), Some("demo-uat-v2"));
}

#[test]
fn loads_mount_declarations_and_dependencies_from_quadlet_only_repo() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Container and mount belong to the same service id (`immich`) so the
    // mount dependency resolves through the catalog. Container goes under
    // quadlet/, mount goes under systemd/ per the payload-kind split.
    let svc_quadlet = services.join("immich/quadlet");
    let svc_systemd = services.join("immich/systemd");
    std::fs::create_dir_all(&svc_quadlet).expect("create quadlet");
    std::fs::create_dir_all(&svc_systemd).expect("create systemd");
    std::fs::write(
        svc_quadlet.join("immich.container"),
        "[Unit]\nDescription=Verification mount-backed service\n\
         After=var-lib-immich-media.mount\nRequires=var-lib-immich-media.mount\n\n\
         [Container]\nImage=docker.io/library/caddy:2.10.2-alpine\nContainerName=immich\n\n\
         [Service]\nRestart=on-failure\nRequiresMountsFor=/var/lib/immich/media\n\n\
         [Install]\nWantedBy=default.target\n",
    )
    .expect("write container");
    std::fs::write(
        svc_systemd.join("var-lib-immich-media.mount"),
        "[Unit]\nDescription=Verification bind mount for reboot resilience\n\n\
         [Mount]\nWhat=/usr/share/zoneinfo\nWhere=/var/lib/immich/media\nType=none\n\
         Options=bind,ro\n\n[X-CoreOps]\nCreateMountpoint=true\n",
    )
    .expect("write mount");
    write_host_yaml(&hosts, "example-host", &["immich"]);
    let rev = git_init_commit(tmp.path());

    let desired = load_with_host(tmp.path(), &rev, "example-host").expect("load desired");

    assert_eq!(desired.mount_declarations.len(), 1);
    let mount = &desired.mount_declarations[0];
    assert_eq!(mount.id, "var-lib-immich-media");
    assert_eq!(mount.target_path, "/var/lib/immich/media");
    assert_eq!(mount.source, "/usr/share/zoneinfo");
    assert_eq!(mount.fstype, "none");
    assert_eq!(
        mount.mount_options,
        vec!["bind".to_string(), "ro".to_string()]
    );
    assert!(!mount.automount);

    assert_eq!(desired.mount_dependencies.len(), 1);
    let dependency = &desired.mount_dependencies[0];
    assert_eq!(dependency.service_name, "immich");
    assert_eq!(dependency.mount_ids, vec!["var-lib-immich-media".to_string()]);
    assert_eq!(
        dependency.consumed_paths,
        vec!["/var/lib/immich/media".to_string()]
    );
}
