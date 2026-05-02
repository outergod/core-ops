use std::fs;
use std::path::{Path, PathBuf};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{init_alpha_repo, HostGuard};
use core_ops::io::repo::HOST_OVERRIDE_ENV;
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;

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
    init_alpha_repo(repo, "example-host")
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
    echo "ActiveState=inactive"
    echo "UnitFileState=disabled"
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
fn reconcile_apply_reports_verification_failure() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_verify");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_verify");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

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
    assert_eq!(result.run.summary, "verification failed");
    assert!(result
        .verification_results
        .iter()
        .any(|res| res.status == core_ops::core::types::VerificationStatus::Failure));
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
