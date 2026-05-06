use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{init_alpha_repo, HostGuard};
use core_ops::io::repo::HOST_OVERRIDE_ENV;
use core_ops::cli::report::build_apply_output;
use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_apply, ReconcileDependencies};
use core_ops::core::types::{
    ConvergenceStatus, DependencyEdgeKind, DeterministicActionClass,
    DeterministicConvergenceRecord, DeterministicPersistedState, DeterministicPlannedAction,
    DeterministicReconciliationPlan, DriftCategory, ManagedObjectKind, NormalizedSnapshot,
    RetainedAppliedSnapshot, RollbackEligibility, RollbackTargetCandidate, SemanticDependencyEdge,
    SemanticDependencyGraph, SemanticDependencyNode, StructuredDriftRecord, VerificationResult,
    VerificationStatus,
};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;
use core_ops::io::state::record_rollback_outcome;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn init_git_repo(repo: &Path) -> String {
    init_alpha_repo(repo, "example-host")
}

fn write_systemctl_stub(dir: &Path, log_path: &Path) -> PathBuf {
    let bin_path = dir.join("systemctl");
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
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
"#,
        log_path.display()
    );
    fs::write(&bin_path, script).expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
    bin_path
}

fn write_recovery_systemctl_stub(dir: &Path, log_path: &Path, active_path: &Path) -> PathBuf {
    let bin_path = dir.join("systemctl");
    let script = format!(
        r#"#!/bin/sh
echo "$@" >> "{}"
case "$1" in
  is-system-running)
    echo "running"
    exit 0
    ;;
  show)
    unit="$2"
    if grep -qxF "$unit" "{}" 2>/dev/null; then
      echo "ActiveState=active"
    else
      echo "ActiveState=failed"
    fi
    echo "UnitFileState=enabled"
    exit 0
    ;;
  start|restart)
    unit="$2"
    {{ grep -vxF "$unit" "{}" 2>/dev/null || true; echo "$unit"; }} > "{}.tmp"
    mv "{}.tmp" "{}"
    exit 0
    ;;
  *)
    exit 0
    ;;
esac
"#,
        log_path.display(),
        active_path.display(),
        active_path.display(),
        active_path.display(),
        active_path.display(),
        active_path.display(),
    );
    fs::write(&bin_path, script).expect("write recovery systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
    bin_path
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

#[test]
fn reconcile_apply_converges_to_desired_state() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_systemctl");
    fs::create_dir_all(&temp).expect("systemctl temp");
    let log_path = temp.join("systemctl.log");
    write_systemctl_stub(&temp, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host");
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

    let result = reconcile_apply(&deps).expect("reconcile apply");

    assert_eq!(result.run.summary, "converged");
}

#[test]
fn reconcile_apply_starts_inactive_unit_when_runtime_recovery_is_required() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let repo = temp_dir("core_ops_repo_recover_apply");
    let rev = init_git_repo(&repo);

    let temp = temp_dir("core_ops_systemctl_recover_apply");
    fs::create_dir_all(&temp).expect("systemctl temp");
    let log_path = temp.join("systemctl.log");
    let active_path = temp.join("active-units");
    write_recovery_systemctl_stub(&temp, &log_path, &active_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", temp.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_recover_apply");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");
    // Pre-staged content must match what init_alpha_repo writes byte-for-byte
    // so the planner sees the unit as already-converged and emits StartUnit
    // (recovery), not WriteQuadlet.
    fs::write(
        host_quadlets.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write converged quadlet");

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

    let result = reconcile_apply(&deps).expect("reconcile apply with recovery");
    let log = fs::read_to_string(&log_path).expect("read systemctl log");

    assert_eq!(result.run.summary, "converged");
    assert!(result.plan.actions.iter().any(|action| action.action_type
        == core_ops::core::types::PlanActionType::StartUnit
        && action.target == "alpha.container"));
    assert!(log.lines().any(|line| line == "start alpha.service"));
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError::new(core_ops::core::types::FailureClass::Apply, err.to_string())
}

#[test]
fn partial_rollback_progress_recording_preserves_target_and_failure_details() {
    let mut state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![RetainedAppliedSnapshot {
            revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            requested_repository: None,
            requested_ref: None,
            snapshot: NormalizedSnapshot {
                revision_id: Some("rev-1".to_string()),
                scope_id: "host:alpha".to_string(),
                objects: Vec::new(),
            },
            retained: true,
        }],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    record_rollback_outcome(
        &mut state,
        RollbackTargetCandidate {
            target_revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            eligibility: RollbackEligibility::Eligible,
            reason: "retained successful snapshot is rollback-eligible".to_string(),
        },
        DeterministicConvergenceRecord {
            desired_revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            status: ConvergenceStatus::Partial,
            attempt_count: 1,
            affected_objects: vec!["alpha.service".to_string()],
            completed_actions: vec!["config:/etc/alpha/env".to_string()],
            failed_actions: vec!["alpha.service".to_string()],
            can_continue: true,
        },
    );

    assert_eq!(
        state
            .latest_rollback_target
            .as_ref()
            .map(|target| target.target_revision_id.as_str()),
        Some("rev-1")
    );
    assert_eq!(
        state
            .latest_convergence
            .as_ref()
            .map(|record| &record.status),
        Some(&ConvergenceStatus::Partial)
    );
    assert_eq!(
        state
            .latest_convergence
            .as_ref()
            .map(|record| record.failed_actions.len()),
        Some(1)
    );
}

#[test]
fn apply_output_reports_failed_blocked_and_skipped_objects() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![
            DeterministicPlannedAction {
                object_id: "alpha.service".to_string(),
                classification: DeterministicActionClass::Update,
                reason: "actual state diverged from desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: [(
                    "contents".to_string(),
                    "desired=[Service]\\nExecStart=/bin/true actual=<absent> applied=<absent>"
                        .to_string(),
                )]
                .into_iter()
                .collect(),
            },
            DeterministicPlannedAction {
                object_id: "beta.service".to_string(),
                classification: DeterministicActionClass::Blocked,
                reason: "blocked prerequisite".to_string(),
                dependency_context: vec!["gamma.service".to_string()],
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "delta.service".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: vec![StructuredDriftRecord {
            object_id: "alpha.service".to_string(),
            category: DriftCategory::ExternalDrift,
            comparison_basis: "desired=last_applied actual!=desired".to_string(),
            auto_action: true,
            attention_required: true,
            details: "desired_fields=1 actual_fields=1".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "alpha.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "alpha.service".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "beta.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "beta.service".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "delta.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "delta.service".to_string(),
                },
            ],
            edges: vec![SemanticDependencyEdge {
                from_object_id: "gamma.service".to_string(),
                to_object_id: "beta.service".to_string(),
                edge_kind: DependencyEdgeKind::Explicit,
                reason: "prerequisite".to_string(),
            }],
        },
    };
    let verification_results = vec![
        VerificationResult {
            target: "alpha.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("verify failed".to_string()),
        },
        VerificationResult {
            target: "beta.service".to_string(),
            status: VerificationStatus::Failure,
            details: Some("blocked: prerequisite unavailable".to_string()),
        },
    ];
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Blocked,
        attempt_count: 1,
        affected_objects: vec![
            "alpha.service".to_string(),
            "beta.service".to_string(),
            "delta.service".to_string(),
        ],
        completed_actions: Vec::new(),
        failed_actions: vec!["alpha.service".to_string(), "beta.service".to_string()],
        can_continue: true,
    };

    let output = build_apply_output(&plan, &verification_results, Some(&convergence));

    assert!(output.events.iter().any(|event| {
        event.object.name == "alpha.service"
            && matches!(
                event.event_kind,
                core_ops::core::types::ExecutionEventKind::ObjectTerminal
            )
            && matches!(event.state, core_ops::core::types::ExecutionState::Failed)
    }));
    assert!(output.events.iter().any(|event| {
        event.object.name == "beta.service"
            && matches!(
                event.event_kind,
                core_ops::core::types::ExecutionEventKind::ObjectTerminal
            )
            && matches!(event.state, core_ops::core::types::ExecutionState::Blocked)
    }));
    assert!(output.events.iter().any(|event| {
        event.object.name == "delta.service"
            && matches!(
                event.event_kind,
                core_ops::core::types::ExecutionEventKind::ObjectTerminal
            )
            && matches!(
                event.state,
                core_ops::core::types::ExecutionState::Unchanged
            )
    }));
}
