use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::HostGuard;
use core_ops::cli::plan as plan_cmd;
use core_ops::cli::report::{build_plan_output, inspect_plan_dependencies};
use core_ops::core::errors::CoreError;
use core_ops::core::evaluate::build_desired_snapshot_from_state;
use core_ops::core::planner::direct_and_transitive_prerequisite_refs;
use core_ops::core::reconcile::reconcile_deterministic_plan;
use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::core::types::{
    Boundaries, BoundaryScope, DeterministicPersistedState, DriftCategory, EnabledState, Invariant,
    ManagedObjectKind, NormalizedManagedObject, NormalizedSnapshot, ObservedUnit, QuadletType,
    RestartPolicy, RetainedAppliedSnapshot, UnitActiveState, Workload,
};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::{build_observed_snapshot, read_observed_state};
use core_ops::io::repo::{load_desired_state, HOST_OVERRIDE_ENV};
use core_ops::io::state::{
    persist_never_run_state, persist_success_state, write_deterministic_state,
    write_persisted_state, STATE_FILE_ENV,
};

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

/// Builds services/alpha/quadlet/alpha.container plus
/// hosts/<host>/host.yaml under the formalized layout, commits the
/// tree, and returns the resulting commit SHA.
fn init_git_repo(repo: &std::path::Path, host: &str) -> String {
    fs::create_dir_all(repo).expect("create repo dir");
    let services = repo.join("services/alpha/quadlet");
    let hosts = repo.join(format!("hosts/{host}"));
    fs::create_dir_all(&services).expect("services");
    fs::create_dir_all(&hosts).expect("hosts");
    fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write quadlet");
    fs::write(
        hosts.join("host.yaml"),
        format!("host: {host}\nservices:\n  - alpha\n"),
    )
    .expect("write host.yaml");
    crate::integration::source_repo_support::git_init_commit(repo)
}

#[test]
fn plan_does_not_apply_changes() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

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

    let result = reconcile_plan(&deps).expect("plan");

    assert_eq!(result.run.summary, "planned");
    assert!(!result.plan.actions.is_empty());
    assert!(std::fs::read_dir(&host_quadlets).unwrap().next().is_none());
}

#[test]
fn cli_plan_summary_uses_deterministic_reconciliation_view() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan_summary");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_summary");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

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

    let output = plan_cmd::plan(&deps, false).expect("cli plan output");
    let summary = strip_ansi(&output.summary);

    assert!(summary.contains("Plan for host "));
    assert!(summary.contains(" @ "));
    assert!(!summary.contains("scope:default"));
    assert!(summary.contains("Summary"));
    assert!(!summary.contains("\n    metadata\n"));
}

#[test]
fn cli_plan_distinguishes_first_run_and_recovery_headers() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let state_dir = temp_dir("core_ops_state_plan_first_run");
    fs::create_dir_all(&state_dir).expect("create state dir");
    let state_file = state_dir.join("status.json");
    let _state_guard = EnvGuard {
        key: STATE_FILE_ENV.to_string(),
        previous: std::env::var_os(STATE_FILE_ENV),
    };
    std::env::set_var(STATE_FILE_ENV, &state_file);
    let repo = temp_dir("core_ops_repo_plan_first_run");
    let rev = init_git_repo(&repo, "example-host");

    let first_run_quadlets = temp_dir("core_ops_host_plan_first_run");
    fs::create_dir_all(&first_run_quadlets).expect("create host quadlets");
    let first_deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&first_run_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &first_run_quadlets, false)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };
    let first_output = plan_cmd::plan(&first_deps, false).expect("first run plan output");
    let first_summary = strip_ansi(&first_output.summary);
    assert!(first_summary.contains("(first run)"));

    let recovery_quadlets = temp_dir("core_ops_host_plan_recovery");
    fs::create_dir_all(&recovery_quadlets).expect("create recovery quadlets");
    fs::write(
        recovery_quadlets.join("alpha.container"),
        "[Container]\nImage=alpine:stale\n",
    )
    .expect("write residual quadlet");
    let recovery_deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&recovery_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &recovery_quadlets, false)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };
    let recovery_output = plan_cmd::plan(&recovery_deps, false).expect("recovery plan output");
    let recovery_summary = strip_ansi(&recovery_output.summary);
    assert!(recovery_summary.contains("(recovery from failed initial apply)"));
}

#[test]
fn cli_plan_exposes_machine_readable_plan_output() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan_machine");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_machine");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

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

    let output = plan_cmd::plan(&deps, false).expect("cli plan output");
    let parsed: serde_json::Value =
        serde_json::from_str(&output.machine).expect("parse machine plan");

    assert_eq!(parsed["view_kind"].as_str(), Some("plan"));
    assert!(parsed["entries"].is_array());
    assert_eq!(
        parsed["revision_context"]["target_revision"].as_str(),
        Some(rev.as_str())
    );
}

#[test]
fn cli_plan_json_stdout_remains_machine_parseable_with_audit_dir() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let repo = temp_dir("core_ops_repo_plan_json_audit");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_json_audit");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let audit_dir = temp_dir("core_ops_plan_audit_dir");
    fs::create_dir_all(&audit_dir).expect("create audit dir");

    let state_file = temp_dir("core_ops_plan_json_audit_state.json");
    persist_never_run_state(&state_file, repo.to_str().expect("repo path"), &rev)
        .expect("write init state");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_core-ops"))
        .arg("plan")
        .arg("--quadlet-dir")
        .arg(host_quadlets.to_str().expect("quadlet dir"))
        .arg("--audit-dir")
        .arg(audit_dir.to_str().expect("audit dir"))
        .arg("--json")
        .env(STATE_FILE_ENV, &state_file)
        .env(HOST_OVERRIDE_ENV, "example-host")
        .output()
        .expect("run core-ops plan");

    assert!(output.status.success(), "plan failed: {:?}", output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("parse stdout json");

    assert_eq!(parsed["view_kind"].as_str(), Some("plan"));
    assert!(!stdout.contains("\naudit "));
}

#[test]
fn cli_plan_uses_host_override_for_scope_when_observed_host_info_is_absent() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let previous = std::env::var_os("CORE_OPS_HOST");
    std::env::set_var("CORE_OPS_HOST", "kadath");
    let _guard = EnvGuard {
        key: "CORE_OPS_HOST".to_string(),
        previous,
    };

    let repo = temp_dir("core_ops_repo_plan_host_scope");
    let rev = init_git_repo(&repo, "kadath");

    let host_quadlets = temp_dir("core_ops_host_plan_host_scope");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

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

    let output = plan_cmd::plan(&deps, false).expect("cli plan output");
    let summary = strip_ansi(&output.summary);

    assert!(summary.contains("Plan for host kadath @ "));
}

#[test]
fn cli_plan_header_uses_persisted_last_applied_revision_when_available() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan_previous_revision");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_previous_revision");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let state_dir = temp_dir("core_ops_state_plan_previous_revision");
    fs::create_dir_all(&state_dir).expect("create state dir");
    let state_file = state_dir.join("status.json");
    persist_success_state(&state_file, repo.to_str().unwrap(), "main", "a1b2c3d")
        .expect("persist state");
    let _guard = EnvGuard {
        key: STATE_FILE_ENV.to_string(),
        previous: std::env::var_os(STATE_FILE_ENV),
    };
    std::env::set_var(STATE_FILE_ENV, &state_file);

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

    let output = plan_cmd::plan(&deps, false).expect("cli plan output");
    let summary = strip_ansi(&output.summary);

    assert!(summary.contains("Plan for host "));
    assert!(summary.contains("a1b2c3d → "));
}

#[test]
fn cli_plan_fails_when_persisted_state_is_unreadable() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan_unreadable_state");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_unreadable_state");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let state_dir = temp_dir("core_ops_state_plan_unreadable_state");
    fs::create_dir_all(&state_dir).expect("create state dir");
    let _guard = EnvGuard {
        key: STATE_FILE_ENV.to_string(),
        previous: std::env::var_os(STATE_FILE_ENV),
    };
    std::env::set_var(STATE_FILE_ENV, &state_dir);

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

    let err = match plan_cmd::plan(&deps, false) {
        Ok(_) => panic!("plan should fail on unreadable state path"),
        Err(err) => err,
    };
    assert!(err.message.contains("failed to read persisted state"));
    assert!(err.message.contains(&state_dir.display().to_string()));
}

#[test]
fn desired_snapshot_extracts_config_and_runtime_dependency_refs() {
    let desired = core_ops::core::types::DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev-1".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads: vec![
            Workload {
                name: "/etc/app.env".to_string(),
                quadlet_type: QuadletType::ConfigFile,
                quadlet_contents: "A=B".to_string(),
                systemd_unit_name: "/etc/app.env".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "app-data".to_string(),
                quadlet_type: QuadletType::Volume,
                quadlet_contents: "[Volume]\nVolumeName=app-data\n".to_string(),
                systemd_unit_name: "app-data.volume".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "app".to_string(),
                quadlet_type: QuadletType::Network,
                quadlet_contents: "[Network]\nNetworkName=app\n".to_string(),
                systemd_unit_name: "app.network".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "app-http".to_string(),
                quadlet_type: QuadletType::Socket,
                quadlet_contents: "[Socket]\nListenStream=8080\n".to_string(),
                systemd_unit_name: "app-http.socket".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            Workload {
                name: "app".to_string(),
                quadlet_type: QuadletType::Container,
                quadlet_contents: "[Container]\nEnvironmentFile=/etc/app.env\nVolume=app-data.volume:/var/lib/app\nNetwork=app\n\n[Service]\nSockets=app-http.socket\n".to_string(),
                systemd_unit_name: "app.container".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
        ],
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: vec!["/etc/app.env".to_string()],
        managed_config_roots: vec!["/etc".to_string()],
        invariants: Vec::new(),
        boundaries: Boundaries { scopes: Vec::new() },
    };

    let snapshot = build_desired_snapshot_from_state(&desired, "host:alpha");
    let container = snapshot
        .objects
        .iter()
        .find(|object| object.object_id == "app.container")
        .expect("container object");

    assert_eq!(
        container.dependency_refs,
        vec![
            "/etc/app.env".to_string(),
            "app-data.volume".to_string(),
            "app-http.socket".to_string(),
            "app.network".to_string(),
        ]
    );
}

#[test]
fn observed_snapshot_matches_desired_snapshot_when_contents_match() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_snapshot_match");
    let rev = init_git_repo(&repo, "example-host");
    let desired = load_desired_state(repo.to_str().unwrap(), &rev).expect("desired state");

    let host_quadlets = temp_dir("core_ops_host_snapshot_match");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");
    for workload in &desired.workloads {
        fs::write(
            host_quadlets.join(&workload.systemd_unit_name),
            &workload.quadlet_contents,
        )
        .expect("write observed workload");
    }

    let observed = read_observed_state(&host_quadlets, Some(&desired), Some("obs".to_string()))
        .expect("observed state");
    let desired_snapshot = build_desired_snapshot_from_state(&desired, "host:alpha");
    let observed_snapshot = build_observed_snapshot(&observed, Some(&desired), "host:alpha");

    assert_eq!(desired_snapshot.objects, observed_snapshot.objects);
}

struct EnvGuard {
    key: String,
    previous: Option<std::ffi::OsString>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(&self.key, value);
        } else {
            std::env::remove_var(&self.key);
        }
    }
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Plan,
        message: err.to_string(),
    }
}

fn object(
    object_id: &str,
    object_kind: ManagedObjectKind,
    material_fields: &[(&str, &str)],
    dependency_refs: &[&str],
) -> NormalizedManagedObject {
    NormalizedManagedObject {
        object_id: object_id.to_string(),
        object_kind,
        material_fields: material_fields
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        dependency_refs: dependency_refs
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

#[test]
fn external_drift_is_classified_and_ordering_is_dependency_aware() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("sha", "abc123")],
                &[],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "ghcr.io/example:1")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };
    let last_applied = desired.clone();
    let actual = NormalizedSnapshot {
        revision_id: Some("obs-1".to_string()),
        scope_id: desired.scope_id.clone(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("sha", "abc123")],
                &[],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[
                    ("unit", "alpha.service"),
                    ("image", "ghcr.io/example:debug"),
                ],
                &["config:/etc/alpha/env"],
            ),
        ],
    };

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");

    assert_eq!(result.plan.actions[0].object_id, "config:/etc/alpha/env");
    assert_eq!(result.plan.actions[1].object_id, "alpha.service");
    assert_eq!(result.plan.drift_records.len(), 1);
    assert_eq!(
        result.plan.drift_records[0].category,
        DriftCategory::ExternalDrift
    );
    assert!(result.plan.actions[1]
        .semantic_diff
        .get("image")
        .expect("image diff")
        .contains("ghcr.io/example:debug"));
}

#[test]
fn dependency_inspection_exposes_prerequisites_dependents_blockers_and_transitive_context() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-3".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/base",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/base")],
                &[],
            ),
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("sha", "abc123")],
                &["config:/etc/alpha/base"],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "ghcr.io/example:1")],
                &["config:/etc/alpha/env", "missing.mount"],
            ),
            object(
                "alpha.timer",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.timer")],
                &["alpha.service"],
            ),
        ],
    };
    let actual = desired.clone();
    let result = reconcile_deterministic_plan(&desired, Some(&desired), &actual)
        .expect("deterministic plan");

    let inspected = inspect_plan_dependencies(&result.plan, "alpha.service");
    let (direct, transitive) =
        direct_and_transitive_prerequisite_refs(&result.plan.graph, "alpha.service");

    let labels = inspected
        .iter()
        .map(|edge| format!("{:?}:{}", edge.relation, edge.object.display_id))
        .collect::<Vec<_>>();
    assert_eq!(direct[0].display_id, "config/etc/alpha/env");
    assert_eq!(transitive[0].display_id, "config/etc/alpha/base");
    assert!(labels.contains(&"Prerequisite:config/etc/alpha/env".to_string()));
    assert!(labels.contains(&"Dependent:service/alpha.timer".to_string()));
    assert!(labels.contains(&"Blocker:mount/missing.mount".to_string()));

    let blocked = result
        .plan
        .actions
        .iter()
        .find(|action| action.object_id == "alpha.service")
        .expect("alpha.service action");
    assert!(blocked
        .dependency_context
        .contains(&"missing.mount".to_string()));
}

#[test]
fn machine_plan_output_retains_dependencies_when_default_human_plan_collapses_unchanged_trees() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("sha", "abc123")],
                &[],
            ),
            object(
                "alpha.network",
                ManagedObjectKind::QuadletResource,
                &[("unit", "alpha-network.service")],
                &[],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service")],
                &["config:/etc/alpha/env", "alpha.network"],
            ),
        ],
    };
    let result = reconcile_deterministic_plan(&desired, Some(&desired), &desired)
        .expect("deterministic plan");
    let rendered = strip_ansi(
        &core_ops::cli::report::format_deterministic_plan_report_with_options(&result.plan, false),
    );
    let machine = build_plan_output(&result.plan);

    assert!(!rendered.contains("requires"));
    let service_entry = machine
        .entries
        .iter()
        .find(|entry| entry.object.display_id == "service/alpha.service")
        .expect("service entry");
    assert_eq!(service_entry.dependencies.len(), 2);
}

#[test]
fn cli_plan_uses_retained_snapshot_baseline_for_expected_deletions() {
    let _lock = path_lock().lock().expect("path lock");
    let state_dir = temp_dir("core_ops_plan_retained_snapshot");
    fs::create_dir_all(&state_dir).expect("create state dir");
    let state_file = state_dir.join("status.json");
    let deterministic_file = state_dir.join("deterministic-state.json");
    std::env::set_var(STATE_FILE_ENV, &state_file);
    struct EnvGuard;
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var(STATE_FILE_ENV);
        }
    }
    let _guard = EnvGuard;

    persist_success_state(&state_file, "repo", "rev-1", "obs").expect("persist success state");
    let observed_workload = Workload {
        name: "whoami".to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: "[Container]\nImage=docker.io/traefik/whoami:v1\n".to_string(),
        systemd_unit_name: "whoami.container".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };
    let observed_state = core_ops::core::types::ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units: Vec::new(),
        workloads: vec![observed_workload.clone()],
        last_reconcile_id: None,
        host_info: Some(core_ops::core::types::HostInfo {
            hostname: "alpha".to_string(),
            os_id: "fedora-coreos".to_string(),
        }),
    };
    let actual = build_observed_snapshot(&observed_state, None, "host:alpha");
    let retained_snapshot = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: actual.scope_id.clone(),
        objects: actual.objects.clone(),
    };
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![],
    };
    write_deterministic_state(
        &deterministic_file,
        &DeterministicPersistedState {
            schema_version: 1,
            current_scope: "host:alpha".to_string(),
            latest_convergence: None,
            latest_rollback_target: None,
            retained_snapshots: vec![RetainedAppliedSnapshot {
                revision_id: "rev-1".to_string(),
                scope_id: "host:alpha".to_string(),
                requested_repository: None,
                requested_ref: None,
                snapshot: retained_snapshot.clone(),
                retained: true,
            }],
        },
    )
    .expect("write deterministic state");
    let deps = ReconcileDependencies {
        load_desired: &|| {
            Ok(core_ops::core::types::DesiredState {
                repository_ref: "repo".to_string(),
                revision_id: "rev-2".to_string(),
                requested_repository: None,
                requested_ref: None,
                workloads: Vec::new(),
                mount_declarations: Vec::new(),
                mount_dependencies: Vec::new(),
                managed_config_paths: Vec::new(),
                managed_config_roots: Vec::new(),
                invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
                boundaries: Boundaries {
                    scopes: vec![BoundaryScope::QuadletSystemd],
                },
            })
        },
        read_observed: &|_| Ok(observed_state.clone()),
        apply_plan: &|_, _| Ok(()),
    };

    let result = reconcile_deterministic_plan(&desired, Some(&retained_snapshot), &actual)
        .expect("sanity deterministic plan");
    assert_eq!(
        result.plan.drift_records[0].category,
        DriftCategory::ExpectedChange
    );

    let output = plan_cmd::plan(&deps, false).expect("cli plan output");
    let summary = strip_ansi(&output.summary);

    assert!(summary.contains("Plan for host alpha @ rev-1 → rev-2"));
    assert!(!summary.contains("(with drift)"));
}

// ── US1 tests: config-file changes trigger dependent container restarts ──────

fn config_desired_state(
    workloads: Vec<Workload>,
    config_paths: Vec<String>,
) -> core_ops::core::types::DesiredState {
    core_ops::core::types::DesiredState {
        repository_ref: "repo".to_string(),
        revision_id: "rev".to_string(),
        requested_repository: None,
        requested_ref: None,
        workloads,
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: config_paths,
        managed_config_roots: Vec::new(),
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: core_ops::core::types::Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn config_observed_state(workloads: Vec<Workload>) -> core_ops::core::types::ObservedState {
    core_ops::core::types::ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units: Vec::new(),
        workloads,
        last_reconcile_id: None,
        host_info: None,
    }
}

fn config_observed_state_with_units(
    workloads: Vec<Workload>,
    units: Vec<ObservedUnit>,
) -> core_ops::core::types::ObservedState {
    core_ops::core::types::ObservedState {
        observed_revision_id: Some("obs".to_string()),
        units,
        workloads,
        last_reconcile_id: None,
        host_info: None,
    }
}

fn active_unit(unit_name: &str) -> ObservedUnit {
    ObservedUnit {
        unit_name: unit_name.to_string(),
        active_state: UnitActiveState::Active,
        enabled_state: EnabledState::Enabled,
    }
}

fn inactive_unit(unit_name: &str) -> ObservedUnit {
    ObservedUnit {
        unit_name: unit_name.to_string(),
        active_state: UnitActiveState::Inactive,
        enabled_state: EnabledState::Enabled,
    }
}

fn config_file_workload(path: &str, contents: &str) -> Workload {
    Workload {
        name: path.to_string(),
        quadlet_type: QuadletType::ConfigFile,
        quadlet_contents: contents.to_string(),
        systemd_unit_name: path.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn container_workload_with_env_file(name: &str, env_file_path: &str) -> Workload {
    Workload {
        name: name.to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: format!(
            "[Container]\nImage=docker.io/example/app:latest\nEnvironmentFile={env_file_path}\n"
        ),
        systemd_unit_name: format!("{name}.container"),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

#[test]
fn config_file_change_schedules_restart_for_dependent_container() {
    let config_path = "/etc/runner/env";
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=new_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Container is active (running) — restart must be scheduled.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    let action_types: Vec<_> = plan
        .actions
        .iter()
        .map(|a| (&a.action_type, a.target.as_str()))
        .collect();

    let restart_pos = plan
        .actions
        .iter()
        .position(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        })
        .unwrap_or_else(|| panic!("expected RestartUnit for app.service, got: {action_types:?}"));

    let write_pos = plan
        .actions
        .iter()
        .position(|a| {
            a.action_type == core_ops::core::types::PlanActionType::WriteQuadlet
                && a.target == config_path
        })
        .expect("expected WriteQuadlet for config file");

    assert!(
        write_pos < restart_pos,
        "WriteQuadlet must precede RestartUnit"
    );
}

#[test]
fn config_file_change_no_restart_when_no_dependents() {
    let config_path = "/etc/runner/env";
    let desired = config_desired_state(
        vec![config_file_workload(config_path, "KEY=new_value")],
        vec![config_path.to_string()],
    );
    let observed = config_observed_state(vec![config_file_workload(config_path, "KEY=old_value")]);

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    let restart_actions: Vec<_> = plan
        .actions
        .iter()
        .filter(|a| a.action_type == core_ops::core::types::PlanActionType::RestartUnit)
        .collect();

    assert!(
        restart_actions.is_empty(),
        "expected no RestartUnit actions when no dependent containers, got: {restart_actions:?}"
    );
}

#[test]
fn config_file_remove_schedules_restart_for_dependent_container() {
    let config_path = "/etc/runner/env";
    // Desired: container only (config file removed)
    let desired = config_desired_state(
        vec![container_workload_with_env_file("app", config_path)],
        vec![], // config removed from desired, so not in managed_config_paths
    );
    // Observed: both config file and container present; container is active (running).
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected RestartUnit for app.service when config file is removed"
    );
}

#[test]
fn config_file_change_no_duplicate_restart_when_container_also_changed() {
    let config_path = "/etc/runner/env";
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=new_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Container also changed independently (different quadlet_contents); container is active.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            Workload {
                name: "app".to_string(),
                quadlet_type: QuadletType::Container,
                quadlet_contents: format!(
                    "[Container]\nImage=docker.io/example/app:v1\nEnvironmentFile={config_path}\n"
                ),
                systemd_unit_name: "app.container".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
        ],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    // actions_for_diff emits RestartUnit("app.container") for the container change;
    // the config-change pass must not add a second RestartUnit for the same workload
    // (regardless of which name — quadlet or runtime — is used for the target).
    let restart_count = plan
        .actions
        .iter()
        .filter(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && (a.target == "app.service" || a.target == "app.container")
        })
        .count();

    assert_eq!(
        restart_count, 1,
        "expected exactly one RestartUnit for app (any form), got {restart_count}"
    );
}

// ── US2 tests: apply report reflects actual restart execution ────────────────

#[test]
fn config_file_change_report_shows_restarted_when_restart_executed() {
    use core_ops::cli::report::build_apply_output;
    use core_ops::core::types::{
        ConvergenceStatus, DeterministicActionClass, DeterministicConvergenceRecord,
        DeterministicPlannedAction, DeterministicReconciliationPlan, ExecutionState,
        SemanticDependencyGraph,
    };
    use std::collections::BTreeMap;

    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev".to_string()),
        baseline_revision_id: Some("base".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:test".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "app.container".to_string(),
            classification: DeterministicActionClass::Restart,
            reason: "config file changed".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: BTreeMap::new(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
    };

    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev".to_string(),
        scope_id: "host:test".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["app.container".to_string()],
        completed_actions: vec!["app.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let output = build_apply_output(&plan, &[], Some(&convergence));

    let terminal = output
        .events
        .iter()
        .find(|e| {
            e.object.name == "app.container"
                && e.event_kind
                    == core_ops::core::types::ExecutionEventKind::ObjectTerminal
        })
        .expect("terminal event for app.container");

    assert_eq!(
        terminal.state,
        ExecutionState::Restarted,
        "expected Restarted state when restart succeeded"
    );
}

#[test]
fn config_file_change_report_shows_failed_when_restart_fails() {
    use core_ops::cli::report::build_apply_output;
    use core_ops::core::types::{
        ConvergenceStatus, DeterministicActionClass, DeterministicConvergenceRecord,
        DeterministicPlannedAction, DeterministicReconciliationPlan, ExecutionState,
        SemanticDependencyGraph,
    };
    use std::collections::BTreeMap;

    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev".to_string()),
        baseline_revision_id: Some("base".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:test".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "app.container".to_string(),
            classification: DeterministicActionClass::Restart,
            reason: "config file changed".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: BTreeMap::new(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
    };

    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev".to_string(),
        scope_id: "host:test".to_string(),
        status: ConvergenceStatus::Failed,
        attempt_count: 1,
        affected_objects: vec!["app.container".to_string()],
        completed_actions: Vec::new(),
        failed_actions: vec!["app.container".to_string()],
        can_continue: false,
    };

    let output = build_apply_output(&plan, &[], Some(&convergence));

    let terminal = output
        .events
        .iter()
        .find(|e| {
            e.object.name == "app.container"
                && e.event_kind
                    == core_ops::core::types::ExecutionEventKind::ObjectTerminal
        })
        .expect("terminal event for app.container");

    assert_eq!(
        terminal.state,
        ExecutionState::Failed,
        "expected Failed state when restart fails, got {:?}",
        terminal.state
    );
}

// ── US3 tests: Add-case edge cases ───────────────────────────────────────────

#[test]
fn config_file_add_restarts_already_running_container() {
    let config_path = "/etc/runner/env";
    // Desired: config file (new) + container with dependency
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Observed: container PRESENT and ACTIVE but config file ABSENT.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![container_workload_with_env_file("app", config_path)],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected RestartUnit for already-running (active) container when config file is added"
    );
}

#[test]
fn config_file_add_no_restart_for_stopped_container() {
    let config_path = "/etc/runner/env";
    // Desired: config file (new) + container with dependency
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Observed: container PRESENT but INACTIVE — intentionally stopped.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![container_workload_with_env_file("app", config_path)],
        vec![inactive_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        !plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected NO RestartUnit for stopped (inactive) container when config file is added"
    );
}

#[test]
fn config_file_remove_schedules_restart_via_volume_dependency() {
    let config_path = "/etc/app/config";
    // Container depends on config via Volume= mount (not EnvironmentFile=)
    let container_with_volume = Workload {
        name: "app".to_string(),
        quadlet_type: QuadletType::Container,
        quadlet_contents: format!(
            "[Container]\nImage=docker.io/example/app:latest\nVolume={config_path}:/cfg:Z\n"
        ),
        systemd_unit_name: "app.container".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };
    // Desired: container only (config file removed)
    let desired = config_desired_state(
        vec![container_with_volume.clone()],
        vec![], // config removed from desired, so not in managed_config_paths
    );
    // Observed: both config file and container present; container is active (running).
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "option=value"),
            container_with_volume,
        ],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected RestartUnit for container with Volume= dependency when config file is removed"
    );
}

#[test]
fn config_file_change_no_duplicate_restart_when_container_changed_via_socket_activation() {
    let config_path = "/etc/runner/env";
    // Socket-activated container: app.socket + app.container both in desired.
    // When app.container changes, actions_for_diff emits RestartUnit(app.service)
    // (runtime name), not RestartUnit(app.container). The config-change pass must
    // normalize to the same key and not add a second restart.
    let socket_workload = Workload {
        name: "app".to_string(),
        quadlet_type: QuadletType::Socket,
        quadlet_contents: "[Socket]\nListenStream=8080\n".to_string(),
        systemd_unit_name: "app.socket".to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    };
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=new_value"),
            container_workload_with_env_file("app", config_path),
            socket_workload.clone(),
        ],
        vec![config_path.to_string()],
    );
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            // Container has old image — triggers a Change diff and RestartUnit(app.service)
            Workload {
                name: "app".to_string(),
                quadlet_type: QuadletType::Container,
                quadlet_contents: format!(
                    "[Container]\nImage=docker.io/example/app:v1\nEnvironmentFile={config_path}\n"
                ),
                systemd_unit_name: "app.container".to_string(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            },
            socket_workload,
        ],
        vec![active_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    let restart_count = plan
        .actions
        .iter()
        .filter(|a| a.action_type == core_ops::core::types::PlanActionType::RestartUnit)
        .filter(|a| a.target == "app.service" || a.target == "app.container")
        .count();

    assert_eq!(
        restart_count, 1,
        "expected exactly one restart for app when container and config both changed (socket-activated case), got {restart_count}"
    );
}

#[test]
fn config_file_change_no_restart_for_stopped_container() {
    let config_path = "/etc/runner/env";
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=new_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Observed: container present but INACTIVE — intentionally stopped.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![inactive_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        !plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected NO RestartUnit for stopped (inactive) container when config file changes"
    );
}

#[test]
fn config_file_remove_no_restart_for_stopped_container() {
    let config_path = "/etc/runner/env";
    // Desired: container only (config file removed)
    let desired = config_desired_state(
        vec![container_workload_with_env_file("app", config_path)],
        vec![],
    );
    // Observed: both present but container INACTIVE — intentionally stopped.
    // ObservedUnit names are runtime names (app.service), not quadlet names (app.container).
    let observed = config_observed_state_with_units(
        vec![
            config_file_workload(config_path, "KEY=old_value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![inactive_unit("app.service")],
    );

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        !plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected NO RestartUnit for stopped (inactive) container when config file is removed"
    );
}

#[test]
fn config_file_add_no_restart_for_new_container() {
    let config_path = "/etc/runner/env";
    // Desired: config file (new) + container (new)
    let desired = config_desired_state(
        vec![
            config_file_workload(config_path, "KEY=value"),
            container_workload_with_env_file("app", config_path),
        ],
        vec![config_path.to_string()],
    );
    // Observed: NEITHER config file nor container
    let observed = config_observed_state(vec![]);

    let plan = core_ops::core::planner::plan(&desired, &observed).expect("plan");

    assert!(
        !plan.actions.iter().any(|a| {
            a.action_type == core_ops::core::types::PlanActionType::RestartUnit
                && a.target == "app.service"
        }),
        "expected NO RestartUnit for new container when config file is added fresh"
    );
}

// T026: plan in Detached state prepends the detached header from the contract
#[test]
fn plan_in_detached_state_prepends_detached_header() {
    use core_ops::core::reconcile::never_run_provenance;
    use core_ops::core::types::{
        ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState, ReconciliationStatus,
        TreeState, PERSISTED_PROVENANCE_SCHEMA_VERSION,
    };

    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_plan_detached");
    let rev = init_git_repo(&repo, "example-host");

    let host_quadlets = temp_dir("core_ops_host_plan_detached");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let state_dir = temp_dir("core_ops_state_plan_detached");
    fs::create_dir_all(&state_dir).expect("create state dir");
    let state_file = state_dir.join("status.json");

    // Build a detached state with last_applied_revision = rev
    let mut reconciliation = never_run_provenance();
    reconciliation.status = ReconciliationStatus::Success;
    reconciliation.last_applied_revision = Some(rev.clone());
    reconciliation.last_attempted_revision = Some(rev.clone());
    reconciliation.last_started_at = Some("2026-01-01T00:00:00Z".to_string());
    reconciliation.last_finished_at = Some("2026-01-01T00:01:00Z".to_string());
    let state = PersistedProvenanceState {
        schema_version: PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: ControllerProvenance {
            version: None,
            revision: None,
            build_time: None,
            tree_state: TreeState::Unknown,
        },
        desired_state: DesiredStateProvenance {
            repository: repo.to_str().unwrap().to_string(),
            requested_ref: "main".to_string(),
            last_observed_revision: None,
            last_observed_at: None,
            layout_version: None,
        },
        reconciliation,
        detached: true,
    };
    write_persisted_state(&state_file, &state).expect("write detached state");

    let _guard = EnvGuard {
        key: STATE_FILE_ENV.to_string(),
        previous: std::env::var_os(STATE_FILE_ENV),
    };
    std::env::set_var(STATE_FILE_ENV, &state_file);

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&host_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
        },
        apply_plan: &|_, _| Ok(()),
    };

    let output = plan_cmd::plan(&deps, false).expect("plan in detached state should succeed");
    let summary = strip_ansi(&output.summary);

    // Verify detached header is prepended
    assert!(
        summary.starts_with("[DETACHED] plan computed against detached revision "),
        "summary should start with detached header, got: {summary}"
    );
    assert!(
        summary.contains(&rev),
        "detached header should contain the detached revision"
    );
    assert!(
        summary.contains("re-attaching to main"),
        "detached header should mention the requested_ref"
    );

    // Verify the plan body is still present after the header
    assert!(
        summary.contains("Plan for "),
        "plan body should follow the detached header"
    );

    let _ = fs::remove_dir_all(state_dir);
    let _ = fs::remove_dir_all(host_quadlets);
    let _ = fs::remove_dir_all(repo);
}
