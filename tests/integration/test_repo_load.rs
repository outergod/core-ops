use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::io::repo::load_desired_state;

use crate::integration::env_lock::path_lock;

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

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_repo_{}", nanos));
    path
}

fn init_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    std::fs::create_dir_all(&quadlets).expect("create quadlets");
    std::fs::write(
        quadlets.join("alpha.container"),
        "[Container]\nImage=alpine",
    )
    .expect("write quadlet");

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

fn init_mount_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    std::fs::create_dir_all(&quadlets).expect("create quadlets");
    std::fs::write(
        quadlets.join("immich.container"),
        "[Unit]\nDescription=Verification mount-backed service\nAfter=var-lib-immich-media.mount\nRequires=var-lib-immich-media.mount\n\n[Container]\nImage=docker.io/library/caddy:2.10.2-alpine\nContainerName=immich\n\n[Service]\nRestart=on-failure\nRequiresMountsFor=/var/lib/immich/media\n\n[Install]\nWantedBy=default.target\n",
    )
    .expect("write container");
    std::fs::write(
        quadlets.join("var-lib-immich-media.mount"),
        "[Unit]\nDescription=Verification bind mount for reboot resilience\n\n[Mount]\nWhat=/usr/share/zoneinfo\nWhere=/var/lib/immich/media\nType=none\nOptions=bind,ro\n\n[X-CoreOps]\nCreateMountpoint=true\n",
    )
    .expect("write mount");

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
        .arg("mount fixture")
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
fn loads_desired_state_from_quadlet_dir() {
    let repo = temp_repo();
    let rev = init_git_repo(&repo);

    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");

    assert_eq!(desired.revision_id, rev);
    assert_eq!(desired.workloads.len(), 1);
    assert_eq!(desired.workloads[0].name, "alpha");
    assert_eq!(desired.workloads[0].systemd_unit_name, "alpha.container");
}

#[test]
fn loads_desired_state_from_git_url_fixture() {
    let repo = temp_repo();
    let rev = init_git_repo(&repo);

    let repo_url = format!("file://{}", repo.display());
    let desired = load_desired_state(&repo_url, &rev).expect("load desired");

    assert!(!desired.workloads.is_empty());
}

#[test]
fn ignores_dotfiles_and_warns_on_unknown_extensions() {
    let repo = temp_repo();
    let rev = init_git_repo(&repo);
    let quadlets = repo.join("quadlets");

    std::fs::write(
        quadlets.join(".ignored.container"),
        "[Container]\nImage=alpine",
    )
    .expect("write dotfile");
    std::fs::write(quadlets.join("readme.txt"), "ignore me").expect("write unknown");

    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");

    assert_eq!(desired.workloads.len(), 1);
    assert_eq!(desired.workloads[0].name, "alpha");
}

#[test]
fn layered_repo_preserves_requested_repository_and_ref() {
    let _lock = path_lock().lock().expect("path lock");
    let previous_host = std::env::var_os("CORE_OPS_HOST");
    let _host_guard = HostGuard(previous_host);
    let repo = temp_repo();
    std::process::Command::new("git")
        .arg("init")
        .arg(&repo)
        .output()
        .expect("git init");
    let services = repo.join("services").join("demo").join("quadlet");
    let hosts = repo.join("hosts").join("uat");
    std::fs::create_dir_all(&services).expect("create services");
    std::fs::create_dir_all(&hosts).expect("create hosts");
    std::fs::write(
        services.join("whoami.container"),
        "[Container]\nImage=quay.io/podman/hello",
    )
    .expect("write service quadlet");
    std::fs::write(hosts.join("host.yaml"), "host: uat\nservices:\n  - demo\n")
        .expect("write host yaml");

    std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .arg("add")
        .arg(".")
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .arg("commit")
        .arg("-m")
        .arg("layered fixture")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .output()
        .expect("git commit");

    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");
    let rev = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .arg("branch")
        .arg("demo-uat-v2")
        .output()
        .expect("git branch");

    std::env::set_var("CORE_OPS_HOST", "uat");
    let desired = load_desired_state(repo.to_str().unwrap(), "demo-uat-v2").expect("load desired");
    // CORE_OPS_HOST is restored by _host_guard on drop

    assert_eq!(desired.revision_id, rev);
    assert_eq!(desired.requested_repository.as_deref(), repo.to_str());
    assert_eq!(desired.requested_ref.as_deref(), Some("demo-uat-v2"));
}

#[test]
fn loads_mount_declarations_and_dependencies_from_quadlet_only_repo() {
    let repo = temp_repo();
    let rev = init_mount_git_repo(&repo);

    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");

    assert_eq!(desired.mount_declarations.len(), 1);
    let mount = &desired.mount_declarations[0];
    assert_eq!(mount.id, "var-lib-immich-media");
    assert_eq!(mount.target_path, "/var/lib/immich/media");
    assert_eq!(mount.source, "/usr/share/zoneinfo");
    assert_eq!(mount.fstype, "none");
    assert_eq!(mount.mount_options, vec!["bind".to_string(), "ro".to_string()]);
    assert!(!mount.automount);

    assert_eq!(desired.mount_dependencies.len(), 1);
    let dependency = &desired.mount_dependencies[0];
    assert_eq!(dependency.service_name, "immich");
    assert_eq!(dependency.mount_ids, vec!["var-lib-immich-media".to_string()]);
    assert_eq!(dependency.consumed_paths, vec!["/var/lib/immich/media".to_string()]);
}
