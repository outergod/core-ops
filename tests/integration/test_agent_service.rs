use std::fs;
use std::path::PathBuf;

use core_ops::cli::agent::{run_agent, AgentConfig};
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
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    fs::create_dir_all(&quadlets).expect("create quadlets");
    fs::write(quadlets.join("alpha.container"), "[Container]\nImage=alpine")
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

fn write_systemctl_stub(dir: &PathBuf) {
    let bin_path = dir.join("systemctl");
    let script = r#"#!/bin/sh
case "$1" in
  is-system-running)
    echo "running"
    exit 0
    ;;
  show)
    echo "ActiveState=active"
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
fn agent_runs_once_with_service_config() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_agent");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_agent");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let quadlet_dir = temp.join("quadlets");
    fs::create_dir_all(&quadlet_dir).expect("quadlet dir");

    let config = AgentConfig {
        repo: repo.display().to_string(),
        rev,
        quadlet_dir,
        audit_dir: None,
        state_file: Some(temp.join("status.json")),
        reload_systemd: true,
        lock_path: Some(temp.join("agent.lock")),
    };

    let output = run_agent(&config).expect("agent run");
    assert!(output.report.contains("actions"));
    assert_eq!(output.run.summary, "converged");
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}
