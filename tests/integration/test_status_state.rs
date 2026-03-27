use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::cli::apply::apply_with_report;
use core_ops::cli::plan as plan_cmd;
use core_ops::cli::status::{
    format_status_text, render_mount_dependency_summary, render_status, render_status_from_path,
};
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::ReconcileDependencies;
use core_ops::core::types::{
    Boundaries, BoundaryScope, DesiredState, Invariant, MountDeclaration, MountDependency,
    MountVerificationMode, PathDependencyMode, VerificationResult, VerificationStatus,
};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;
use core_ops::io::state::{
    persist_success_state, read_persisted_state, resolve_state_file, DETERMINISTIC_STATE_FILE_NAME,
    STATE_FILE_ENV,
};

fn fixture(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/fixtures/provenance_state").join(name);
    fs::read_to_string(path).expect("read fixture")
}

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{prefix}_{nanos}"));
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

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Plan,
        message: err.to_string(),
    }
}

struct EnvGuard {
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_state_file(path: &Path) -> Self {
        let previous = std::env::var_os(STATE_FILE_ENV);
        std::env::set_var(STATE_FILE_ENV, path);
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(STATE_FILE_ENV, value);
        } else {
            std::env::remove_var(STATE_FILE_ENV);
        }
    }
}

#[test]
fn status_output_reflects_canonical_success_snapshot_contents() {
    let output = format_status_text(&fixture("valid-success.json"));

    assert!(output.starts_with("provenance\n"));
    assert!(output.contains("\"repository\": \"file:///var/lib/core-ops/repo\""));
    assert!(output.contains("\"requested_ref\": \"main\""));
    assert!(output.contains("\"status\": \"success\""));
}

#[test]
fn status_output_reflects_never_run_snapshot_contents() {
    let output = format_status_text(&fixture("valid-never-run.json"));

    assert!(output.contains("\"status\": \"never_run\""));
    assert!(output.contains("\"generation\": 0"));
}

#[test]
fn status_output_reflects_in_progress_snapshot_contents() {
    let output = format_status_text(
        r#"{
  "schema_version": 1,
  "controller": {
    "version": "0.1.0",
    "revision": "8f3c2ab",
    "build_time": "2026-03-23T10:00:00Z",
    "tree_state": "clean"
  },
  "desired_state": {
    "repository": "file:///var/lib/core-ops/repo",
    "requested_ref": "main",
    "last_observed_revision": "c98dd10",
    "last_observed_at": "2026-03-23T10:07:00Z"
  },
  "reconciliation": {
    "generation": 12,
    "status": "in_progress",
    "running": true,
    "last_attempted_revision": "c98dd10",
    "last_applied_revision": "a42be91",
    "last_started_at": "2026-03-23T10:07:01Z",
    "last_finished_at": null,
    "attempted_observed_divergence": null
  }
}"#,
    );

    assert!(output.contains("\"status\": \"in_progress\""));
    assert!(output.contains("\"running\": true"));
    assert!(output.contains("\"last_applied_revision\": \"a42be91\""));
}

#[test]
fn status_output_is_stable_for_unchanged_snapshot_contents() {
    let contents = fixture("valid-success.json");

    let first = format_status_text(&contents);
    let second = format_status_text(&contents);

    assert_eq!(first, second);
}

#[test]
fn status_output_reports_absent_for_invalid_or_missing_snapshot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let invalid = root.join("tests/fixtures/provenance_state/invalid-partial.json");
    let missing = root.join("tests/fixtures/provenance_state/missing.json");

    let invalid_output = render_status_from_path(&invalid);
    let missing_output = render_status_from_path(&missing);

    assert!(invalid_output.contains("\"status\": \"absent\""));
    assert!(missing_output.contains("\"status\": \"absent\""));
}

#[test]
fn status_output_rebuilds_after_invalid_snapshot_is_replaced() {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "core_ops_status_rebuild_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix epoch")
            .as_nanos()
    ));
    fs::write(&path, "{\n  \"schema_version\": 1,\n  \"controller\": {").expect("write invalid");

    let absent = render_status_from_path(&path);
    assert!(absent.contains("\"status\": \"absent\""));

    persist_success_state(&path, "file:///var/lib/core-ops/repo", "main", "deadbeef")
        .expect("rebuild snapshot");
    let rebuilt = render_status_from_path(&path);

    assert!(rebuilt.contains("\"status\": \"success\""));
    assert!(rebuilt.contains("\"last_applied_revision\": \"deadbeef\""));

    let _ = fs::remove_file(path);
}

#[test]
fn apply_creates_state_snapshot_on_first_run_from_implicit_path() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_dir("core_ops_repo_status_default_apply");
    let rev = init_git_repo(&repo);
    let temp = temp_dir("core_ops_status_default_apply");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", temp.display(), old_path));
    let _path_guard = PathGuard { previous: old_path };

    let state_path = temp.join("status.json");
    let _state_guard = EnvGuard::set_state_file(&state_path);
    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

    let implicit_state_path = resolve_state_file(None);
    let (_result, _report, _plan) = apply_with_report(
        repo.to_str().unwrap(),
        &rev,
        &host_quadlets,
        false,
        Some(implicit_state_path),
    )
    .expect("apply");

    assert!(state_path.exists());
    let state = read_persisted_state(&state_path)
        .expect("read state")
        .expect("state exists");
    assert_eq!(
        state.reconciliation.status,
        core_ops::core::types::ReconciliationStatus::Success
    );
}

#[test]
fn plan_does_not_create_state_snapshot_from_implicit_path() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_dir("core_ops_repo_status_plan");
    let rev = init_git_repo(&repo);
    let temp = temp_dir("core_ops_status_plan");
    fs::create_dir_all(&temp).expect("temp dir");

    let state_path = temp.join("status.json");
    let _state_guard = EnvGuard::set_state_file(&state_path);
    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&host_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &host_quadlets, false)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };

    let output = plan_cmd::plan(&deps, false).expect("plan");
    assert!(output.summary.contains("Plan for host "));
    assert!(!state_path.exists());
}

#[test]
fn apply_can_explicitly_opt_out_of_state_persistence() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_dir("core_ops_repo_status_no_state");
    let rev = init_git_repo(&repo);
    let temp = temp_dir("core_ops_status_no_state");
    fs::create_dir_all(&temp).expect("temp dir");
    write_systemctl_stub(&temp);

    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", temp.display(), old_path));
    let _path_guard = PathGuard { previous: old_path };

    let state_path = temp.join("status.json");
    let _state_guard = EnvGuard::set_state_file(&state_path);
    let host_quadlets = temp.join("host_quadlets");
    fs::create_dir_all(&host_quadlets).expect("host quadlets");

    let (_result, report, _plan) =
        apply_with_report(repo.to_str().unwrap(), &rev, &host_quadlets, false, None)
            .expect("apply");

    assert!(!state_path.exists());
    assert!(!report.contains("\"status\": \"success\""));
}

#[test]
fn status_uses_implicit_state_path_when_no_explicit_path_is_given() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let state_path = temp_dir("core_ops_status_implicit").join("status.json");
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(&state_path, fixture("valid-success.json")).expect("write state");
    let _state_guard = EnvGuard::set_state_file(&state_path);

    let output = render_status(None);
    assert!(output.contains("\"status\": \"success\""));

    let _ = fs::remove_file(state_path);
}

#[test]
fn status_appends_deterministic_convergence_and_rollback_summaries_when_present() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let state_path = temp_dir("core_ops_status_deterministic").join("status.json");
    let state_dir = state_path.parent().expect("state dir").to_path_buf();
    fs::create_dir_all(&state_dir).expect("create state dir");
    fs::write(&state_path, fixture("valid-success.json")).expect("write state");
    fs::write(
        state_dir.join(DETERMINISTIC_STATE_FILE_NAME),
        r#"{
  "schema_version": 1,
  "current_scope": "host:alpha",
  "retained_snapshots": [],
  "latest_convergence": {
    "desired_revision_id": "rev-2",
    "scope_id": "host:alpha",
    "status": "repeated_failure",
    "attempt_count": 3,
    "affected_objects": ["alpha.service"],
    "completed_actions": ["config:/etc/alpha/env"],
    "failed_actions": ["alpha.service"],
    "can_continue": false
  },
  "latest_rollback_target": {
    "target_revision_id": "rev-1",
    "scope_id": "host:alpha",
    "eligibility": "eligible",
    "reason": "retained successful snapshot is rollback-eligible"
  }
}"#,
    )
    .expect("write deterministic state");

    let output = render_status(Some(state_path.clone()));
    assert!(output.contains("convergence scope=host:alpha status=repeated_failure"));
    assert!(output.contains("rollback target=rev-1 eligibility=eligible"));

    let _ = fs::remove_file(state_path);
    let _ = fs::remove_file(state_dir.join(DETERMINISTIC_STATE_FILE_NAME));
}

#[test]
fn mount_status_summary_reports_dependency_counts_and_failures() {
    let desired = DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        workloads: Vec::new(),
        mount_declarations: vec![MountDeclaration {
            id: "immich-media".to_string(),
            target_path: "/srv/immich/media".to_string(),
            source: "nas:/media".to_string(),
            fstype: "nfs".to_string(),
            mount_options: vec!["rw".to_string()],
            network_backed: true,
            automount: true,
            verification_mode: MountVerificationMode::UnitAndPath,
            prepared_path: None,
        }],
        mount_dependencies: vec![MountDependency {
            service_name: "immich".to_string(),
            mount_ids: vec!["immich-media".to_string()],
            consumed_paths: vec!["/srv/immich/media".to_string()],
            path_dependency_mode: PathDependencyMode::RequiresMountsFor,
            unit_dependency_mode: core_ops::core::types::UnitDependencyMode::AfterAndRequires,
        }],
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    };
    let verification_results = vec![
        VerificationResult {
            target: "srv-immich-media.automount".to_string(),
            status: VerificationStatus::Success,
            details: None,
        },
        VerificationResult {
            target: "srv-immich-media.mount".to_string(),
            status: VerificationStatus::Failure,
            details: Some("degraded: mount target not mounted".to_string()),
        },
    ];

    let summary =
        render_mount_dependency_summary(&desired, &verification_results).expect("mount summary");

    assert_eq!(summary, "mounts refs=1 dependencies=1 verification_failures=1");
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}
