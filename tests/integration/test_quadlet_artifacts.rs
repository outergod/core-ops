use std::fs;
use std::path::PathBuf;

use crate::integration::env_lock::path_lock;
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;
use core_ops::io::systemd::SYSTEMD_UNIT_DIR_ENV;

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
    fs::write(
        quadlets.join("alpha.container"),
        "[Container]\nImage=alpine",
    )
    .expect("write container");
    fs::write(quadlets.join("beta.socket"), "[Socket]\nListenStream=8080").expect("write socket");
    fs::write(quadlets.join("gamma.volume"), "[Volume]\nDriver=local").expect("write volume");

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
fn reconcile_apply_supports_socket_and_volume_quadlets() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_artifacts");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_artifacts");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");
    let systemd_units = temp.join("systemd_units");
    fs::create_dir_all(&systemd_units).expect("systemd units");
    let _systemd_guard = EnvGuard::set(SYSTEMD_UNIT_DIR_ENV, &systemd_units);

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&host_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &host_quadlets, true)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };

    let result = reconcile_apply(&deps).expect("apply");
    assert_eq!(result.run.summary, "converged");

    assert!(host_quadlets.join("alpha.container").exists());
    assert!(systemd_units.join("beta.socket").exists());
    assert!(host_quadlets.join("gamma.volume").exists());
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Apply,
        message: err.to_string(),
    }
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

struct EnvGuard {
    key: String,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(key: &str, value: &PathBuf) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self {
            key: key.to_string(),
            previous,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(&self.key, value),
            None => std::env::remove_var(&self.key),
        }
    }
}
