//! Shared scaffolding for integration tests that drive
//! `core_ops::io::repo::load_desired_state` against the formalized
//! source-repository layout introduced by spec 016.
//!
//! Tests using this module set `CORE_OPS_HOST` and read live working
//! directories, so each test must hold `path_lock()` for the duration
//! of the load. `HostGuard` restores the prior value on drop.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use core_ops::core::types::DesiredState;
use core_ops::io::repo::{load_desired_state, RepoError, HOST_OVERRIDE_ENV};

use crate::integration::env_lock::path_lock;

const EXAMPLES_DIR: &str = "specs/016-source-repository-layout/examples";

pub struct HostGuard(Option<OsString>);

impl HostGuard {
    pub fn capture() -> Self {
        Self(std::env::var_os(HOST_OVERRIDE_ENV))
    }
}

impl Drop for HostGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            std::env::set_var(HOST_OVERRIDE_ENV, value);
        } else {
            std::env::remove_var(HOST_OVERRIDE_ENV);
        }
    }
}

pub fn examples_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(EXAMPLES_DIR)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub fn git_init_commit(repo: &Path) -> String {
    Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["add", "."])
        .output()
        .expect("git add");
    let commit = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["commit", "-m", "fixture", "--allow-empty"])
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .output()
        .expect("git commit");
    assert!(
        commit.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let head = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse");
    String::from_utf8_lossy(&head.stdout).trim().to_string()
}

pub fn materialize_example(name: &str) -> (TempDir, String) {
    let tmp = TempDir::new().expect("tempdir");
    copy_dir_recursive(&examples_root().join(name), tmp.path()).expect("copy example");
    let rev = git_init_commit(tmp.path());
    (tmp, rev)
}

pub fn materialize_skeleton() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let services = tmp.path().join("services");
    let hosts = tmp.path().join("hosts");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::create_dir_all(&hosts).expect("hosts dir");
    (tmp, services, hosts)
}

pub fn write_host_yaml(hosts_dir: &Path, host: &str, services: &[&str]) {
    let dir = hosts_dir.join(host);
    std::fs::create_dir_all(&dir).expect("host dir");
    let mut body = format!("host: {host}\nservices:\n");
    for svc in services {
        body.push_str(&format!("  - {svc}\n"));
    }
    std::fs::write(dir.join("host.yaml"), body).expect("host.yaml");
}

/// Builds a minimal services/alpha/quadlet/alpha.container fixture
/// with `[Container]\nImage=alpine\n` plus hosts/<host>/host.yaml
/// selecting `alpha`, then commits the tree under `repo` (creating
/// `repo` first if it does not exist) and returns the resulting
/// commit SHA. This is the canonical alpha-fixture every "is the
/// loader/planner/applier basically working?" integration test
/// builds; per-test variations should inline their own setup.
pub fn init_alpha_repo(repo: &Path, host: &str) -> String {
    std::fs::create_dir_all(repo).expect("create repo dir");
    let services = repo.join("services/alpha/quadlet");
    let hosts = repo.join(format!("hosts/{host}"));
    std::fs::create_dir_all(&services).expect("services");
    std::fs::create_dir_all(&hosts).expect("hosts");
    std::fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write quadlet");
    std::fs::write(
        hosts.join("host.yaml"),
        format!("host: {host}\nservices:\n  - alpha\n"),
    )
    .expect("write host.yaml");
    git_init_commit(repo)
}

pub fn load_with_host(repo: &Path, rev: &str, host: &str) -> Result<DesiredState, RepoError> {
    load_source_with_host(repo.to_str().expect("utf-8 path"), rev, host)
}

pub fn load_source_with_host(
    repo_source: &str,
    rev: &str,
    host: &str,
) -> Result<DesiredState, RepoError> {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, host);
    load_desired_state(repo_source, rev)
}
