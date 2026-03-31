use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::integration::env_lock::path_lock;
use core_ops::cli::report::{build_explain_output, format_explain_output_report};
use core_ops::io::state::{DETERMINISTIC_STATE_FILE_NAME, STATE_FILE_ENV};

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

fn write_systemctl_stub(dir: &Path) {
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
    assert_eq!(first.result.run.summary, "converged");

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
    assert_eq!(
        second.result.run.status,
        core_ops::core::types::RunStatus::Failure
    );

    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&state_file).expect("read failed state snapshot"))
            .expect("parse failed snapshot");

    assert_eq!(
        snapshot["desired_state"]["repository"].as_str(),
        Some(repo.to_str().expect("repo path"))
    );
    assert_eq!(
        snapshot["desired_state"]["requested_ref"].as_str(),
        Some("main")
    );
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
    assert_eq!(
        snapshot["reconciliation"]["status"].as_str(),
        Some("failed")
    );
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
    assert_eq!(result.result.run.summary, "converged");

    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&state_file).expect("read snapshot"))
            .expect("parse snapshot");

    let desired_state = snapshot["desired_state"]
        .as_object()
        .expect("desired_state object");
    assert_eq!(desired_state.len(), 4);
    assert_eq!(
        desired_state
            .get("last_observed_revision")
            .and_then(Value::as_str),
        Some(revision.as_str())
    );
    assert!(snapshot.get("desired_state_by_target").is_none());
    assert!(snapshot.get("targets").is_none());
}

#[test]
fn deterministic_apply_persists_convergence_state_next_to_status_snapshot() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_deterministic_state");
    let revision = init_git_repo(&repo);

    let temp = temp_dir("core_ops_deterministic_state");
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

    let output = core_ops::cli::apply::apply_with_report(
        repo.to_str().expect("repo path"),
        "main",
        &host_quadlets,
        true,
        Some(state_file.clone()),
    )
    .expect("apply");
    assert_eq!(output.result.run.summary, "converged");

    let deterministic_state_path = state_file
        .parent()
        .expect("state parent")
        .join(DETERMINISTIC_STATE_FILE_NAME);
    let deterministic_snapshot: Value = serde_json::from_str(
        &fs::read_to_string(&deterministic_state_path).expect("read deterministic snapshot"),
    )
    .expect("parse deterministic snapshot");

    assert_eq!(deterministic_snapshot["schema_version"].as_u64(), Some(1));
    assert_eq!(
        deterministic_snapshot["latest_convergence"]["desired_revision_id"].as_str(),
        Some(revision.as_str())
    );
    assert_eq!(
        deterministic_snapshot["latest_convergence"]["status"].as_str(),
        Some("success")
    );
    assert_eq!(
        deterministic_snapshot["latest_convergence"]["can_continue"].as_bool(),
        Some(true)
    );
    assert_eq!(
        deterministic_snapshot["current_scope"],
        deterministic_snapshot["latest_convergence"]["scope_id"]
    );
    assert_eq!(
        deterministic_snapshot["retained_snapshots"]
            .as_array()
            .map(|entries| entries.len()),
        Some(1)
    );
    assert_eq!(
        deterministic_snapshot["retained_snapshots"][0]["revision_id"].as_str(),
        Some(revision.as_str())
    );
    assert_eq!(
        deterministic_snapshot["retained_snapshots"][0]["retained"].as_bool(),
        Some(true)
    );
}

#[test]
fn apply_report_uses_retained_baseline_for_unchanged_reruns() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_baseline");
    let revision = init_git_repo(&repo);

    let temp = temp_dir("core_ops_apply_baseline");
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
    assert_eq!(first.result.run.summary, "converged");

    let second = core_ops::cli::apply::apply_with_report(
        repo.to_str().expect("repo path"),
        "main",
        &host_quadlets,
        true,
        Some(state_file),
    )
    .expect("second apply");

    assert!(second.human_report.contains("1 unchanged"));
    assert!(!second.human_report.contains("created"));
    assert!(!second.human_report.contains("updated"));
    assert!(!second.human_report.contains("restarted"));

    let parsed: Value =
        serde_json::from_str(&second.machine_report).expect("parse second apply output");
    assert_eq!(
        parsed["revision_context"]["target_revision"].as_str(),
        Some(revision.as_str())
    );
    assert_eq!(parsed["summary"]["changed_count"].as_u64(), Some(0));
    assert_eq!(parsed["summary"]["unchanged_count"].as_u64(), Some(1));
    assert_eq!(parsed["events"][0]["state"].as_str(), Some("unchanged"));
    assert_eq!(parsed["events"][0]["action"].as_str(), Some("no_op"));
}

#[test]
fn explain_output_supports_single_object_inspection_with_mount_metadata() {
    let plan = core_ops::core::types::DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "var-lib-demo.mount".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::Recover,
                reason: "runtime reconciliation required: unit not active: failed".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "var-lib-demo.automount".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "desired, last applied, and actual state already match".to_string(),
                dependency_context: vec!["var-lib-demo.mount".to_string()],
                semantic_diff: Default::default(),
            },
        ],
        drift_records: vec![core_ops::core::types::StructuredDriftRecord {
            object_id: "var-lib-demo.mount".to_string(),
            category: core_ops::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: "runtime reconciliation required: unit not active: failed".to_string(),
        }],
        graph: core_ops::core::types::SemanticDependencyGraph {
            nodes: vec![
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "var-lib-demo.mount".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::Mount,
                    ordering_key: "var-lib-demo.mount".to_string(),
                },
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "var-lib-demo.automount".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::Automount,
                    ordering_key: "var-lib-demo.automount".to_string(),
                },
            ],
            edges: vec![core_ops::core::types::SemanticDependencyEdge {
                from_object_id: "var-lib-demo.mount".to_string(),
                to_object_id: "var-lib-demo.automount".to_string(),
                edge_kind: core_ops::core::types::DependencyEdgeKind::Explicit,
                reason: "paired automount".to_string(),
            }],
        },
    };
    let explain =
        build_explain_output(&plan, &[], None, "mount/var-lib-demo.mount").expect("explain output");
    let json = serde_json::to_value(&explain).expect("serialize explain");

    assert_eq!(json["view_kind"].as_str(), Some("explain"));
    assert_eq!(
        json["object"]["display_id"].as_str(),
        Some("mount/var-lib-demo.mount")
    );
    assert_eq!(
        json["metadata"]["runtime_unit"].as_str(),
        Some("var-lib-demo.mount")
    );
    assert_eq!(
        json["metadata"]["automount_unit"].as_str(),
        Some("var-lib-demo.automount")
    );
    assert_eq!(json["x_coreops"]["CreateMountpoint"].as_bool(), Some(true));
}

#[test]
fn humane_explain_output_prioritizes_state_reason_intent_and_identity() {
    let plan = core_ops::core::types::DeterministicReconciliationPlan {
        desired_revision_id: Some("454ac5f18bd1290826debbc344ac6ee5c71cffb6".to_string()),
        baseline_revision_id: Some("221145e64cfab3d9a8f093f4d44f3dd24f4fdad1".to_string()),
        requested_repository: None,
        requested_ref: Some("demo-uat-v2".to_string()),
        last_applied_requested_repository: None,
        last_applied_requested_ref: Some("demo-uat-v1".to_string()),
        scope_id: "host:core-ops-uat".to_string(),
        actions: vec![
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "etc-caddy-Caddyfile.config".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "declarative state matches desired state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "frontend.network".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "declarative state matches desired state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "frontend-data.volume".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "declarative state matches desired state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            core_ops::core::types::DeterministicPlannedAction {
                object_id: "whoami.container".to_string(),
                classification: core_ops::core::types::DeterministicActionClass::NoOp,
                reason: "declarative state matches desired state".to_string(),
                dependency_context: vec![
                    "etc-caddy-Caddyfile.config".to_string(),
                    "frontend.network".to_string(),
                    "frontend-data.volume".to_string(),
                ],
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: core_ops::core::types::SemanticDependencyGraph {
            nodes: vec![
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "etc-caddy-Caddyfile.config".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::RenderedArtifact,
                    ordering_key: "etc-caddy-Caddyfile.config".to_string(),
                },
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "frontend.network".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend.network".to_string(),
                },
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "frontend-data.volume".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend-data.volume".to_string(),
                },
                core_ops::core::types::SemanticDependencyNode {
                    object_id: "whoami.container".to_string(),
                    object_kind: core_ops::core::types::ManagedObjectKind::QuadletResource,
                    ordering_key: "whoami.container".to_string(),
                },
            ],
            edges: vec![
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "etc-caddy-Caddyfile.config".to_string(),
                    to_object_id: "whoami.container".to_string(),
                    edge_kind: core_ops::core::types::DependencyEdgeKind::Explicit,
                    reason: "container requires config".to_string(),
                },
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "frontend.network".to_string(),
                    to_object_id: "whoami.container".to_string(),
                    edge_kind: core_ops::core::types::DependencyEdgeKind::Explicit,
                    reason: "container requires network".to_string(),
                },
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "frontend-data.volume".to_string(),
                    to_object_id: "whoami.container".to_string(),
                    edge_kind: core_ops::core::types::DependencyEdgeKind::Explicit,
                    reason: "container requires volume".to_string(),
                },
            ],
        },
    };
    let explain = build_explain_output(&plan, &[], None, "container/whoami.container")
        .expect("explain output");
    let rendered = format_explain_output_report(&explain);

    assert!(rendered.contains("Explain: container/whoami.container"));
    assert!(rendered.contains("State"));
    assert!(rendered.contains("Action: unchanged"));
    assert!(rendered.contains("Reason: declarative state matches desired state"));
    assert!(rendered.contains("Apply intent: no action planned"));
    assert!(rendered.contains("Context"));
    assert!(rendered.contains("Host:   core-ops-uat"));
    assert!(rendered.contains("Target: 454ac5f1 (demo-uat-v2)"));
    assert!(rendered.contains("Last:   221145e6 (demo-uat-v1)"));
    assert!(rendered.contains("Identity"));
    assert!(rendered.contains("Object: container/whoami.container"));
    assert!(rendered.contains("Type:   container"));
    assert!(rendered.contains("Unit:   whoami.service"));
    assert!(rendered.contains("Dependency context"));
    assert!(rendered.contains("Requires"));
    assert!(rendered.contains("network/frontend.network"));
    assert!(rendered.contains("volume/frontend-data.volume"));
    assert!(rendered.contains(
        "network/frontend.network\n  │  state: unchanged\n  │  reason: declarative state matches desired state"
    ));
    assert!(rendered.contains("state: unchanged"));
    assert!(rendered.contains("reason: declarative state matches desired state"));
    assert!(rendered.contains("Summary"));
    assert!(rendered.contains(
        "container/whoami.container is unchanged and has no planned reconciliation action."
    ));
    assert!(!rendered.contains("action_or_outcome:"));
    assert!(!rendered.contains("\nMetadata\n"));
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
