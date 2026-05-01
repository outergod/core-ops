use std::fs;
use std::path::{Path, PathBuf};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::HostGuard;
use core_ops::cli::agent::{run_agent, AgentConfig, AgentExitReason};
use core_ops::io::repo::HOST_OVERRIDE_ENV;
use core_ops::io::state::persist_never_run_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn init_git_repo(repo: &Path) -> String {
    fs::create_dir_all(repo).expect("create repo dir");
    let services = repo.join("services/alpha/quadlet");
    let hosts = repo.join("hosts/example-host");
    fs::create_dir_all(&services).expect("services");
    fs::create_dir_all(&hosts).expect("hosts");
    fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write quadlet");
    fs::write(
        hosts.join("host.yaml"),
        "host: example-host\nservices:\n  - alpha\n",
    )
    .expect("write host.yaml");
    crate::integration::source_repo_support::git_init_commit(repo)
}

fn write_systemctl_stub(dir: &Path) {
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
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
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

    let state_file = temp.join("status.json");
    persist_never_run_state(&state_file, &repo.display().to_string(), &rev)
        .expect("persist init state");

    let config = AgentConfig {
        quadlet_dir,
        audit_dir: None,
        state_file: Some(state_file),
        reload_systemd: true,
        lock_path: Some(temp.join("agent.lock")),
    };

    let result = run_agent(&config).expect("agent run");
    let output = match result {
        AgentExitReason::Completed(o) => o,
        AgentExitReason::Uninitialized => panic!("agent exited as uninitialized"),
        AgentExitReason::Detached { revision } => panic!("agent exited as detached at {revision}"),
    };
    assert!(output.report.contains("Apply for host"));
    assert!(output.report.contains("Execution"));
    assert!(!output.run.summary.is_empty());
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}
