use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use core_ops::io::state::STATE_FILE_ENV;
use crate::integration::env_lock::path_lock;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn init_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(repo)
        .output()
        .expect("git init");
    commit_quadlet(repo, "[Container]\nImage=alpine:3.19\n")
}

fn commit_quadlet(repo: &PathBuf, contents: &str) -> String {
    let quadlets = repo.join("quadlets");
    fs::create_dir_all(&quadlets).expect("create quadlets");
    fs::write(quadlets.join("alpha.container"), contents).expect("write quadlet");

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

fn write_systemctl_stub(dir: &PathBuf) {
    let bin_path = dir.join("systemctl");
    let script = r#"#!/bin/sh
FAIL_MARKER="${0}.fail"
case "$1" in
  is-system-running)
    echo "running"
    exit 0
    ;;
  show)
    if [ -f "$FAIL_MARKER" ]; then
      echo "ActiveState=failed"
    else
      echo "ActiveState=active"
    fi
    echo "UnitFileState=enabled"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#;
    fs::write(&bin_path, script).expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
}

#[test]
fn failed_reconciliation_preserves_last_applied_revision_and_desired_state_fields() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_provenance");
    let first_revision = init_git_repo(&repo);

    let temp = temp_dir("core_ops_provenance");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let state_file = temp.join("status.json");
    std::env::set_var(STATE_FILE_ENV, &state_file);
    let _state_guard = StateFileGuard;

    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

    let first = core_ops::cli::apply::apply_with_report(
        repo.to_str().expect("repo path"),
        "main",
        &host_quadlets,
        true,
        Some(state_file.clone()),
    )
    .expect("first apply");
    assert_eq!(first.0.run.summary, "converged");

    let second_revision = commit_quadlet(&repo, "[Container]\nImage=alpine:3.20\n");
    let fail_marker = temp.join("systemctl.fail");
    fs::write(&fail_marker, "").expect("write fail marker");

    let second = core_ops::cli::apply::apply_with_report(
        repo.to_str().expect("repo path"),
        "main",
        &host_quadlets,
        true,
        Some(state_file.clone()),
    )
    .expect("second apply");
    assert_eq!(second.0.run.status, core_ops::core::types::RunStatus::Failure);

    let snapshot: Value = serde_json::from_str(
        &fs::read_to_string(&state_file).expect("read failed state snapshot"),
    )
    .expect("parse failed snapshot");

    assert_eq!(
        snapshot["desired_state"]["repository"].as_str(),
        Some(repo.to_str().expect("repo path"))
    );
    assert_eq!(snapshot["desired_state"]["requested_ref"].as_str(), Some("main"));
    assert_eq!(
        snapshot["desired_state"]["last_observed_revision"].as_str(),
        Some(second_revision.as_str())
    );
    assert_eq!(
        snapshot["reconciliation"]["last_attempted_revision"].as_str(),
        Some(second_revision.as_str())
    );
    assert_eq!(
        snapshot["reconciliation"]["last_applied_revision"].as_str(),
        Some(first_revision.as_str())
    );
    assert_eq!(snapshot["reconciliation"]["status"].as_str(), Some("failed"));
    assert_eq!(snapshot["reconciliation"]["generation"].as_u64(), Some(2));
}

#[test]
fn desired_state_provenance_remains_host_scoped() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_host_scope");
    let revision = init_git_repo(&repo);

    let temp = temp_dir("core_ops_host_scope");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let state_file = temp.join("status.json");
    std::env::set_var(STATE_FILE_ENV, &state_file);
    let _state_guard = StateFileGuard;

    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

    let result = core_ops::cli::apply::apply_with_report(
        repo.to_str().expect("repo path"),
        "main",
        &host_quadlets,
        true,
        Some(state_file.clone()),
    )
    .expect("apply");
    assert_eq!(result.0.run.summary, "converged");

    let snapshot: Value = serde_json::from_str(
        &fs::read_to_string(&state_file).expect("read snapshot"),
    )
    .expect("parse snapshot");

    let desired_state = snapshot["desired_state"]
        .as_object()
        .expect("desired_state object");
    assert_eq!(desired_state.len(), 4);
    assert_eq!(
        desired_state.get("last_observed_revision").and_then(Value::as_str),
        Some(revision.as_str())
    );
    assert!(snapshot.get("desired_state_by_target").is_none());
    assert!(snapshot.get("targets").is_none());
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

struct StateFileGuard;

impl Drop for StateFileGuard {
    fn drop(&mut self) {
        std::env::remove_var(STATE_FILE_ENV);
    }
}
