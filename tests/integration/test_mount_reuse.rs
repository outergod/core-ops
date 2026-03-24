use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::io::repo::load_desired_state;

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_mount_reuse_{}", nanos));
    path
}

fn init_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    std::fs::create_dir_all(repo.join("services/immich")).expect("service dir");
    std::fs::create_dir_all(repo.join("hosts/alpha")).expect("host alpha");
    std::fs::create_dir_all(repo.join("hosts/beta/overrides")).expect("host beta");
    std::fs::create_dir_all(repo.join("hosts/gamma/overrides")).expect("host gamma");
    std::fs::create_dir_all(repo.join("hosts/invalid/overrides")).expect("host invalid");

    std::fs::write(
        repo.join("services/immich/immich.container"),
        "[Container]\nImage=immich\n",
    )
    .expect("write artifact");
    std::fs::write(
        repo.join("services/immich/service.yaml"),
        r#"
mounts:
  - id: immich-media
    target_path: /var/lib/immich/media
    source: nas:/volume1/media
    fstype: nfs
    network_backed: true
    ownership_scope: [immich]
  - id: immich-cache
    target_path: /var/lib/immich/cache
    source: /var/cache/immich
    fstype: none
    ownership_scope: [immich]
requires_mounts:
  - immich-media
"#,
    )
    .expect("write service yaml");

    for host in ["alpha", "beta", "gamma", "invalid"] {
        std::fs::write(
            repo.join(format!("hosts/{host}/host.yaml")),
            format!("host: {host}\nservices:\n  - immich\n"),
        )
        .expect("write host");
    }

    std::fs::write(
        repo.join("hosts/beta/overrides/mounts.yaml"),
        r#"
mounts:
  - id: immich-media
    target_path: /srv/immich/media
    source: nas:/volume2/media
    fstype: nfs
    network_backed: true
    automount: true
    ownership_scope: [immich]
"#,
    )
    .expect("write beta overrides");

    std::fs::write(
        repo.join("hosts/gamma/overrides/mounts.yaml"),
        r#"
service_mounts:
  immich:
    - immich-cache
"#,
    )
    .expect("write gamma overrides");

    std::fs::write(
        repo.join("hosts/invalid/overrides/mounts.yaml"),
        r#"
mounts:
  - id: unknown-mount
    target_path: /srv/invalid
    source: nas:/invalid
    fstype: nfs
    network_backed: true
    ownership_scope: [immich]
"#,
    )
    .expect("write invalid overrides");

    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("add")
        .arg(".")
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("commit")
        .arg("-m")
        .arg("fixture")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .output()
        .expect("git commit");

    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn reusable_mount_declarations_support_host_overrides_and_dependency_switches() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_repo();
    let rev = init_repo(&repo);

    let previous = std::env::var_os("CORE_OPS_HOST");
    let _guard = HostGuard(previous);
    std::env::set_var("CORE_OPS_HOST", "alpha");
    let alpha = load_desired_state(repo.to_str().expect("repo str"), &rev).expect("load alpha");
    assert!(alpha
        .workloads
        .iter()
        .any(|workload| workload.systemd_unit_name == "var-lib-immich-media.mount"));
    let alpha_service = alpha
        .workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == "immich.container")
        .expect("alpha service");
    assert!(alpha_service
        .quadlet_contents
        .contains("RequiresMountsFor=/var/lib/immich/media"));

    std::env::set_var("CORE_OPS_HOST", "beta");
    let beta = load_desired_state(repo.to_str().expect("repo str"), &rev).expect("load beta");
    assert!(beta
        .workloads
        .iter()
        .any(|workload| workload.systemd_unit_name == "srv-immich-media.automount"));
    let beta_service = beta
        .workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == "immich.container")
        .expect("beta service");
    assert!(beta_service
        .quadlet_contents
        .contains("RequiresMountsFor=/srv/immich/media"));

    std::env::set_var("CORE_OPS_HOST", "gamma");
    let gamma = load_desired_state(repo.to_str().expect("repo str"), &rev).expect("load gamma");
    let gamma_service = gamma
        .workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == "immich.container")
        .expect("gamma service");
    assert!(gamma_service
        .quadlet_contents
        .contains("RequiresMountsFor=/var/lib/immich/cache"));
    assert!(!gamma_service
        .quadlet_contents
        .contains("RequiresMountsFor=/var/lib/immich/media"));

    std::env::set_var("CORE_OPS_HOST", "invalid");
    let err = load_desired_state(repo.to_str().expect("repo str"), &rev).expect_err("invalid override");
    assert!(err.to_string().contains("host mount override outside selected services"));
}

struct HostGuard(Option<std::ffi::OsString>);

impl Drop for HostGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            std::env::set_var("CORE_OPS_HOST", value);
        } else {
            std::env::remove_var("CORE_OPS_HOST");
        }
    }
}
