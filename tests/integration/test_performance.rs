use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{git_init_commit, HostGuard};
use core_ops::io::repo::HOST_OVERRIDE_ENV;
use core_ops::cli::report::{
    build_result_output, format_deterministic_plan_report, format_result_output_report,
};
use core_ops::core::errors::CoreError;
use core_ops::core::evaluate::build_desired_snapshot_from_state;
use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::core::verify::verify_state;
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::build_observed_snapshot;
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

/// Builds a single `bench` service holding `count` container artifacts
/// under services/bench/quadlet/, plus hosts/example-host/host.yaml
/// selecting that service. Returns the resulting commit SHA.
fn init_git_repo(repo: &Path, count: usize) -> String {
    fs::create_dir_all(repo).expect("create repo");
    let svc = repo.join("services/bench/quadlet");
    let hosts = repo.join("hosts/example-host");
    fs::create_dir_all(&svc).expect("services");
    fs::create_dir_all(&hosts).expect("hosts");
    for idx in 0..count {
        fs::write(
            svc.join(format!("workload{idx}.container")),
            "[Container]\nImage=alpine\n",
        )
        .expect("write quadlet");
    }
    fs::write(
        hosts.join("host.yaml"),
        "host: example-host\nservices:\n  - bench\n",
    )
    .expect("write host.yaml");
    git_init_commit(repo)
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
fn reconcile_apply_completes_under_budget() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_perf");
    let rev = init_git_repo(&repo, 50);

    let temp = temp_dir("core_ops_perf");
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

    let start = Instant::now();
    let result = reconcile_apply(&deps).expect("apply");
    let elapsed = start.elapsed();

    assert_eq!(result.run.summary, "converged");
    assert!(elapsed.as_secs() < 120);
}

#[test]
fn plan_and_result_rendering_complete_within_interactive_budget() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_render_perf");
    let rev = init_git_repo(&repo, 50);

    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("load desired");
    let observed_dir = repo.join("empty-observed");
    fs::create_dir_all(&observed_dir).expect("create observed dir");
    let observed = read_observed_state(&observed_dir, Some(&desired), Some("obs".to_string()))
        .expect("observed");
    let scope_id = "host:alpha".to_string();
    let desired_snapshot = build_desired_snapshot_from_state(&desired, &scope_id);
    let observed_snapshot = build_observed_snapshot(&observed, Some(&desired), &scope_id);
    let verification = verify_state(&desired, &observed);
    let deterministic = core_ops::core::reconcile::reconcile_deterministic_plan_with_runtime(
        &desired_snapshot,
        None,
        &observed_snapshot,
        &verification,
    )
    .expect("deterministic");

    let start = Instant::now();
    let _plan_text = format_deterministic_plan_report(&deterministic.plan);
    let result_view = build_result_output(&deterministic.plan, &verification, None);
    let _result_text = format_result_output_report(&result_view);
    let elapsed = start.elapsed();

    assert!(elapsed.as_secs_f64() < 1.0, "rendering took {:?}", elapsed);
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
