use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::io::repo::load_desired_state;

fn temp_repo() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("core_ops_layered_overrides_{}", nanos));
    path
}

fn copy_dir_all(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create dir");
    for entry in std::fs::read_dir(src).expect("read dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest);
        } else {
            std::fs::copy(&path, &dest).expect("copy file");
        }
    }
}

fn init_layered_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let fixtures = PathBuf::from("tests/fixtures/layered_overrides");
    copy_dir_all(&fixtures.join("services"), &repo.join("services"));
    copy_dir_all(&fixtures.join("hosts"), &repo.join("hosts"));

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
fn applies_host_overrides_after_base_dropins() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_repo();
    let rev = init_layered_repo(&repo);

    let previous = std::env::var_os("CORE_OPS_HOST");
    let _guard = HostGuard(previous);
    std::env::set_var("CORE_OPS_HOST", "kadath");
    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");

    let traefik = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.container")
        .expect("traefik container");

    assert!(traefik.quadlet_contents.contains("TRAEFIK_LOG_LEVEL=INFO"));
    assert!(traefik.quadlet_contents.contains("TRAEFIK_HOST=kadath"));

    let base_pos = traefik
        .quadlet_contents
        .find("TRAEFIK_LOG_LEVEL=INFO")
        .expect("base drop-in");
    let host_pos = traefik
        .quadlet_contents
        .find("TRAEFIK_HOST=kadath")
        .expect("host drop-in");
    assert!(base_pos < host_pos);

    let traefik_socket = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.socket")
        .expect("traefik socket");

    assert!(traefik_socket.quadlet_contents.contains("ListenStream=127.0.0.1:8080"));
    assert!(!traefik_socket.quadlet_contents.contains("ListenStream=127.0.0.1:8081"));

    let socket_base_dropin = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.socket.d/10-defaults.conf")
        .expect("socket base drop-in");
    assert!(socket_base_dropin.quadlet_contents.contains("NoDelay=true"));

    let socket_host_dropin = desired
        .workloads
        .iter()
        .find(|w| w.systemd_unit_name == "traefik.socket.d/20-host.conf")
        .expect("socket host drop-in");
    assert!(socket_host_dropin.quadlet_contents.contains("ListenStream=127.0.0.1:8081"));
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
