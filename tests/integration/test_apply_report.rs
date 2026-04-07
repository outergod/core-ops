use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::integration::env_lock::path_lock;
use core_ops::cli::apply::{apply_with_report, apply_with_report_streaming};
use core_ops::cli::report::{
    build_apply_output, build_result_output, format_apply_output_report,
    format_result_output_report, render_apply_output_from_events, ApplyHumanMode,
    ApplyInteractiveEvent, ApplyProgressRenderer, ApplyRunDisplayState,
};
use core_ops::core::types::{
    ConvergenceStatus, DependencyEdgeKind, DeterministicActionClass,
    DeterministicConvergenceRecord, DeterministicPlannedAction, DeterministicReconciliationPlan,
    ManagedObjectKind, SemanticDependencyGraph, SemanticDependencyNode, StructuredDriftRecord,
    VerificationResult, VerificationStatus,
};
use core_ops::io::apply::ApplyError;
use serde_json::Value;

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

fn init_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    std::fs::create_dir_all(&quadlets).expect("create quadlets");
    std::fs::write(
        quadlets.join("alpha.container"),
        "[Container]\nImage=alpine",
    )
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

fn write_systemctl_stub(dir: &Path, log_path: &Path) -> PathBuf {
    let bin_path = dir.join("systemctl");
    let script = format!(
        "#!/bin/sh\n\n\
echo \"$@\" >> \"{}\"\n\
exit 0\n",
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

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

#[test]
fn apply_report_uses_phase_aware_human_and_machine_output() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_report");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_report");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_apply_report");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let output = apply_with_report(repo.to_str().unwrap(), &rev, &host_quadlets, false, None)
        .expect("apply report");

    assert!(output.human_report.contains("Apply for host"));
    assert!(!output.human_report.contains("Phases"));
    assert!(output.human_report.contains("Execution"));
    assert!(output.human_report.contains("Summary"));
    assert!(!output.human_report.contains('{'));
    let parsed: Value = serde_json::from_str(&output.machine_report).expect("parse apply json");
    assert_eq!(parsed["view_kind"].as_str(), Some("apply"));
    assert!(parsed["phases"].is_array());
    assert!(parsed["events"].is_array());
    assert!(parsed["summary"].is_object());
    assert!(parsed["events"]
        .as_array()
        .expect("events array")
        .iter()
        .any(|event| event["event_kind"] == "object_terminal"));
    assert!(parsed.get("execution").is_none());
    assert!(parsed.get("objects").is_none());
    assert!(!output.result.run.summary.is_empty());
}

#[test]
fn apply_report_only_narrates_phases_in_verbose_mode() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_narration");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_narration");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_apply_narration");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let output = apply_with_report(repo.to_str().unwrap(), &rev, &host_quadlets, false, None)
        .expect("apply report");

    assert!(!output.human_report.contains("Phases"));
    assert!(output.verbose_report.contains("Phases"));
    let resolution = output
        .verbose_report
        .find("Resolving desired state\tcompleted")
        .expect("resolution line");
    let planning = output
        .verbose_report
        .find("Planning\tcompleted")
        .expect("planning line");
    let execution = output
        .verbose_report
        .find("Applying\tcompleted")
        .expect("execution line");
    let convergence = output
        .verbose_report
        .find("Verifying\t")
        .expect("convergence line");
    let summary = output
        .verbose_report
        .find("Summary")
        .expect("summary heading");

    assert!(resolution < planning);
    assert!(planning < execution);
    assert!(execution < convergence);
    assert!(convergence < summary);
}

#[test]
fn apply_report_distinguishes_first_run_and_recovery_headers() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_first_run");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_first_run");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let first_run_quadlets = temp_dir("core_ops_host_apply_first_run");
    fs::create_dir_all(&first_run_quadlets).expect("create host quadlets");
    let first_run_output = apply_with_report(
        repo.to_str().unwrap(),
        &rev,
        &first_run_quadlets,
        false,
        None,
    )
    .expect("first run apply report");
    assert!(first_run_output.human_report.contains("(first run)"));

    let recovery_quadlets = temp_dir("core_ops_host_apply_recovery");
    fs::create_dir_all(&recovery_quadlets).expect("create recovery quadlets");
    fs::write(
        recovery_quadlets.join("alpha.container"),
        "[Container]\nImage=alpine:stale\n",
    )
    .expect("write residual quadlet");
    let recovery_output = apply_with_report(
        repo.to_str().unwrap(),
        &rev,
        &recovery_quadlets,
        false,
        None,
    )
    .expect("recovery apply report");
    assert!(recovery_output
        .human_report
        .contains("(recovery from failed initial apply)"));
}

#[test]
fn apply_report_surfaces_verification_failures_for_unchanged_objects() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: None,
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::NoOp,
            reason: "no change".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: vec![StructuredDriftRecord {
            object_id: "frontend.container".to_string(),
            category: core_ops::core::types::DriftCategory::ExpectedChange,
            comparison_basis: "desired=actual".to_string(),
            auto_action: false,
            attention_required: true,
            details: "verification_pending".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };
    let verification_results = vec![VerificationResult {
        target: "frontend.container".to_string(),
        status: VerificationStatus::Failure,
        details: Some("unit is inactive".to_string()),
    }];
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::RepeatedFailure,
        attempt_count: 3,
        affected_objects: vec!["frontend.container".to_string()],
        completed_actions: Vec::new(),
        failed_actions: vec!["frontend.container".to_string()],
        can_continue: false,
    };

    let rendered = format_apply_output_report(
        &plan,
        &verification_results,
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Recovery,
    );

    assert!(rendered.contains("container/frontend.container"));
    assert!(rendered.contains("failed"));
    assert!(rendered.contains("unit is inactive"));
    assert!(rendered.contains("failed during Verifying"));
    assert!(rendered.contains("suggested checks"));
    assert!(rendered.contains("core-ops explain container/frontend.container"));
    assert!(rendered.contains("systemctl status frontend.service"));
    assert!(rendered.contains("journalctl -u frontend.service -b"));
    assert!(rendered.contains("Outcome: non-converging"));
}

#[test]
fn apply_streaming_report_emits_progress_then_terminal_lines() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_streaming");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_streaming");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_apply_streaming");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let mut transcript = String::new();
    let output = apply_with_report_streaming(
        repo.to_str().unwrap(),
        &rev,
        &host_quadlets,
        false,
        None,
        ApplyHumanMode::Default,
        |chunk| transcript.push_str(chunk),
    )
    .expect("streaming apply report");

    let creating = transcript.find("creating...").expect("progress line");
    let created = transcript.find("created").expect("terminal line");
    let summary = transcript.find("Summary").expect("summary heading");

    assert!(transcript.contains("Apply for host"));
    assert!(transcript.contains("Execution"));
    assert!(transcript.contains("container/alpha.container"));
    assert!(creating < created);
    assert!(created < summary);
    assert!(output.machine_report.contains("\"view_kind\":\"apply\""));
}

#[test]
fn apply_report_fails_when_persisted_state_path_is_unreadable() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_dir("core_ops_repo_apply_unreadable_state");
    let rev = init_git_repo(&repo);

    let systemctl = temp_dir("core_ops_systemctl_apply_unreadable_state");
    fs::create_dir_all(&systemctl).expect("systemctl dir");
    let log_path = systemctl.join("systemctl.log");
    write_systemctl_stub(&systemctl, &log_path);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", systemctl.display(), old_path);
    std::env::set_var("PATH", new_path);
    let _guard = PathGuard { previous: old_path };

    let host_quadlets = temp_dir("core_ops_host_apply_unreadable_state");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let state_dir = temp_dir("core_ops_state_apply_unreadable_state");
    fs::create_dir_all(&state_dir).expect("create state dir");

    let err = match apply_with_report(
        repo.to_str().unwrap(),
        &rev,
        &host_quadlets,
        false,
        Some(state_dir.clone()),
    ) {
        Ok(_) => panic!("apply should fail on unreadable state path"),
        Err(err) => err,
    };

    assert!(err.message.contains("failed to read persisted state"));
    assert!(err.message.contains(&state_dir.display().to_string()));
}

#[test]
fn apply_report_renders_recovery_actions_as_recovered() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::Recover,
            reason: "runtime reconciliation required: unit not active: failed".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: vec![StructuredDriftRecord {
            object_id: "frontend.container".to_string(),
            category: core_ops::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: "runtime reconciliation required: unit not active: failed".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["frontend.container".to_string()],
        completed_actions: vec!["frontend.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let rendered = format_apply_output_report(
        &plan,
        &[],
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );
    let plain = strip_ansi(&rendered);

    assert!(rendered.contains("container/frontend.container"));
    assert!(rendered.contains("recovered"));
    assert!(rendered.contains("because runtime reconciliation required"));
    assert!(rendered.contains("1 recover"));
    assert!(plain.contains("\n\nSummary\n"));
    assert!(rendered.contains("Outcome: converged"));
}

#[test]
fn apply_report_hides_unchanged_prerequisites_for_recovery_actions_by_default() {
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
                object_id: "config:/etc/caddy/Caddyfile".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "frontend.network".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "frontend-data.volume".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "frontend.container".to_string(),
                classification: DeterministicActionClass::Recover,
                reason: "runtime reconciliation required: unit not active: Failed".to_string(),
                dependency_context: vec![
                    "config:/etc/caddy/Caddyfile".to_string(),
                    "frontend.network".to_string(),
                    "frontend-data.volume".to_string(),
                ],
                semantic_diff: Default::default(),
            },
        ],
        drift_records: vec![StructuredDriftRecord {
            object_id: "frontend.container".to_string(),
            category: core_ops::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: "runtime reconciliation required: unit not active: Failed".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "config:/etc/caddy/Caddyfile".to_string(),
                    object_kind: ManagedObjectKind::RenderedArtifact,
                    ordering_key: "config:/etc/caddy/Caddyfile".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "frontend.network".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend.network".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "frontend-data.volume".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend-data.volume".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "frontend.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend.container".to_string(),
                },
            ],
            edges: vec![
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "frontend.container".to_string(),
                    to_object_id: "config:/etc/caddy/Caddyfile".to_string(),
                    edge_kind: DependencyEdgeKind::Explicit,
                    reason: "declared dependency".to_string(),
                },
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "frontend.container".to_string(),
                    to_object_id: "frontend.network".to_string(),
                    edge_kind: DependencyEdgeKind::Explicit,
                    reason: "declared dependency".to_string(),
                },
                core_ops::core::types::SemanticDependencyEdge {
                    from_object_id: "frontend.container".to_string(),
                    to_object_id: "frontend-data.volume".to_string(),
                    edge_kind: DependencyEdgeKind::Explicit,
                    reason: "declared dependency".to_string(),
                },
            ],
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["frontend.container".to_string()],
        completed_actions: vec!["frontend.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let rendered = format_apply_output_report(
        &plan,
        &[],
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );

    assert!(rendered.contains("container/frontend.container"));
    assert!(rendered.contains("because runtime reconciliation required"));
    assert!(!rendered.contains("    requires\n"));
}

#[test]
fn apply_header_renders_requested_ref_secondarily_when_meaningful() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("454ac5f18bd1290826debbc344ac6ee5c71cffb6".to_string()),
        baseline_revision_id: Some("221145e64cfab3d9a8f093f4d44f3dd24f4fdad1".to_string()),
        requested_repository: None,
        requested_ref: Some("demo-uat-v2".to_string()),
        last_applied_requested_repository: None,
        last_applied_requested_ref: Some("demo-uat-v1".to_string()),
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::NoOp,
            reason: "no change".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };

    let rendered = format_apply_output_report(
        &plan,
        &[],
        None,
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );

    assert!(strip_ansi(&rendered)
        .contains("Apply for host alpha @ 221145e6 (demo-uat-v1) → 454ac5f1 (demo-uat-v2)"));
}

#[test]
fn apply_error_display_omits_empty_systemd_error_suffix() {
    assert_eq!(
        ApplyError::SystemdCommandFailed(String::new()).to_string(),
        "systemd command failed"
    );
}

#[test]
fn apply_streaming_failure_omits_bare_systemd_error_stub() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::Update,
            reason: "actual state diverged".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };

    let mut renderer = ApplyProgressRenderer::new(
        &plan,
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );
    let rendered = renderer
        .render_failed("frontend.container", "systemd command failed:")
        .expect("rendered failure");

    assert!(!rendered.contains("systemd command failed:"));
    assert!(!rendered.contains("systemd command failed\n"));
    assert!(rendered.contains("failed during Applying"));
    assert!(rendered.contains("suggested checks"));
    assert!(rendered.contains("core-ops explain container/frontend.container"));
}

#[test]
fn apply_interactive_started_event_uses_spinner_line_without_dependency_block() {
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
                object_id: "frontend.container".to_string(),
                classification: DeterministicActionClass::Update,
                reason: "actual state diverged".to_string(),
                dependency_context: vec!["network/frontend.network".to_string()],
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "network/frontend.network".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "matches desired state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "frontend.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend.container".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "network/frontend.network".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "network/frontend.network".to_string(),
                },
            ],
            edges: vec![core_ops::core::types::SemanticDependencyEdge {
                from_object_id: "frontend.container".to_string(),
                to_object_id: "network/frontend.network".to_string(),
                edge_kind: DependencyEdgeKind::Explicit,
                reason: "declared dependency".to_string(),
            }],
        },
    };

    let mut renderer = ApplyProgressRenderer::new(
        &plan,
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );
    let event = renderer
        .render_started_interactive("frontend.container")
        .expect("interactive start event");

    match event {
        ApplyInteractiveEvent::Started { line, .. } => {
            assert!(line.contains("container/frontend.container"));
            assert!(line.contains("updating..."));
            assert!(!line.contains("requires"));
            assert!(!line.contains("network/frontend.network"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn apply_interactive_finish_event_starts_on_a_new_line_before_summary() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "alpha.container".to_string(),
            classification: DeterministicActionClass::Create,
            reason: "missing from actual state".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "alpha.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "alpha.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };

    let mut renderer = ApplyProgressRenderer::new(
        &plan,
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );
    let event = renderer.finish_interactive(&[], None);

    match event {
        ApplyInteractiveEvent::Finish(text) => {
            let stripped = strip_ansi(&text);
            assert!(stripped.starts_with('\n'));
            assert!(stripped.contains("\nSummary\n"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn result_view_preserves_plan_and_apply_continuity() {
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
                object_id: "config:/etc/demo.conf".to_string(),
                classification: DeterministicActionClass::Update,
                reason: "actual state diverged from desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: [(
                    "contents".to_string(),
                    "desired=foo=2 actual=foo=1 applied=foo=1".to_string(),
                )]
                .into_iter()
                .collect(),
            },
            DeterministicPlannedAction {
                object_id: "frontend.container".to_string(),
                classification: DeterministicActionClass::Recover,
                reason: "runtime reconciliation required: unit not active: Failed".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: vec![StructuredDriftRecord {
            object_id: "frontend.container".to_string(),
            category: core_ops::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: "runtime reconciliation required: unit not active: Failed".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "config:/etc/demo.conf".to_string(),
                    object_kind: ManagedObjectKind::RenderedArtifact,
                    ordering_key: "config:/etc/demo.conf".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "frontend.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "frontend.container".to_string(),
                },
            ],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec![
            "config:/etc/demo.conf".to_string(),
            "frontend.container".to_string(),
        ],
        completed_actions: vec![
            "config:/etc/demo.conf".to_string(),
            "frontend.container".to_string(),
        ],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let apply = build_apply_output(&plan, &[], Some(&convergence));
    let result = build_result_output(&plan, &[], Some(&convergence));
    let rendered = format_result_output_report(&result);

    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.entries[0].object.display_id, "config/etc/demo.conf");
    assert_eq!(
        result.entries[1].object.display_id,
        "container/frontend.container"
    );
    assert_eq!(
        apply.events[2].object.display_id,
        result.entries[0].object.display_id
    );
    assert_eq!(
        apply.events[5].object.display_id,
        result.entries[1].object.display_id
    );
    assert!(rendered.contains("Result for host alpha @ rev-1 → rev-2"));
    assert!(rendered.contains("updated"));
    assert!(rendered.contains("recovered"));
    assert!(rendered.contains("Outcome: converged"));
}

#[test]
fn structured_apply_events_can_be_replayed_into_humane_output() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::Update,
            reason: "actual state diverged from desired snapshot".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["frontend.container".to_string()],
        completed_actions: vec!["frontend.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };
    let apply = build_apply_output(&plan, &[], Some(&convergence));
    let replayed = render_apply_output_from_events(&apply, ApplyHumanMode::Default);

    assert!(replayed.contains("Apply for host alpha @ rev-1 → rev-2"));
    assert!(replayed.contains("container/frontend.container"));
    assert!(replayed.contains("updated"));
    assert!(replayed.contains("Outcome: converged"));
}

#[test]
fn apply_report_marks_recover_as_failed_when_convergence_failed_actions_use_runtime_unit_names() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "frontend.container".to_string(),
            classification: DeterministicActionClass::Recover,
            reason: "runtime reconciliation required: unit not active: Failed".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        }],
        drift_records: vec![StructuredDriftRecord {
            object_id: "frontend.container".to_string(),
            category: core_ops::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: "runtime reconciliation required: unit not active: Failed".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![SemanticDependencyNode {
                object_id: "frontend.container".to_string(),
                object_kind: ManagedObjectKind::QuadletResource,
                ordering_key: "frontend.container".to_string(),
            }],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::RepeatedFailure,
        attempt_count: 3,
        affected_objects: vec!["frontend.service".to_string()],
        completed_actions: Vec::new(),
        failed_actions: vec!["frontend.service".to_string()],
        can_continue: false,
    };

    let rendered = format_apply_output_report(
        &plan,
        &[],
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );

    assert!(rendered.contains("container/frontend.container"));
    assert!(rendered.contains("failed"));
    assert!(!rendered.contains("recovered"));
    assert!(rendered.contains("failed during Verifying"));
    assert!(rendered.contains("suggested checks"));
    assert!(rendered.contains("core-ops explain container/frontend.container"));
}

#[test]
fn apply_summary_pluralizes_create_counts_consistently() {
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
                object_id: "alpha.container".to_string(),
                classification: DeterministicActionClass::Create,
                reason: "missing from actual state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "beta.container".to_string(),
                classification: DeterministicActionClass::Create,
                reason: "missing from actual state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "alpha.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "alpha.container".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "beta.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "beta.container".to_string(),
                },
            ],
            edges: Vec::new(),
        },
    };
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::Success,
        attempt_count: 1,
        affected_objects: vec!["alpha.container".to_string(), "beta.container".to_string()],
        completed_actions: vec!["alpha.container".to_string(), "beta.container".to_string()],
        failed_actions: Vec::new(),
        can_continue: true,
    };

    let rendered = format_apply_output_report(
        &plan,
        &[],
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    );

    assert!(rendered.contains("2 creates"));
}

#[test]
fn apply_output_humane_and_json_preserve_same_semantics() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        requested_repository: Some("file:///var/lib/core-ops/repo".to_string()),
        requested_ref: Some("demo-uat-v2".to_string()),
        last_applied_requested_repository: Some("file:///var/lib/core-ops/repo".to_string()),
        last_applied_requested_ref: Some("demo-uat-v1".to_string()),
        scope_id: "host:alpha".to_string(),
        actions: vec![
            DeterministicPlannedAction {
                object_id: "alpha.container".to_string(),
                classification: DeterministicActionClass::Create,
                reason: "missing from actual state".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "beta.container".to_string(),
                classification: DeterministicActionClass::NoOp,
                reason: "no change".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: vec![StructuredDriftRecord {
            object_id: "beta.container".to_string(),
            category: core_ops::core::types::DriftCategory::ExpectedChange,
            comparison_basis: "desired=actual".to_string(),
            auto_action: false,
            attention_required: true,
            details: "verification_pending".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "alpha.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "alpha.container".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "beta.container".to_string(),
                    object_kind: ManagedObjectKind::QuadletResource,
                    ordering_key: "beta.container".to_string(),
                },
            ],
            edges: Vec::new(),
        },
    };
    let verification_results = vec![VerificationResult {
        target: "beta.container".to_string(),
        status: VerificationStatus::Failure,
        details: Some("unit is inactive".to_string()),
    }];
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::RepeatedFailure,
        attempt_count: 1,
        affected_objects: vec!["beta.container".to_string()],
        completed_actions: vec!["alpha.container".to_string()],
        failed_actions: vec!["beta.container".to_string()],
        can_continue: false,
    };

    let human = strip_ansi(&format_apply_output_report(
        &plan,
        &verification_results,
        Some(&convergence),
        ApplyHumanMode::Default,
        ApplyRunDisplayState::Managed,
    ));
    let json =
        serde_json::to_value(build_apply_output(&plan, &verification_results, Some(&convergence)))
            .expect("serialize apply output");

    assert!(human.contains("rev-2"));
    assert!(human.contains("demo-uat-v2"));
    assert!(human.contains("1 create"));
    assert!(human.contains("1 failed"));
    assert!(human.contains("Outcome: non-converging"));
    assert!(human.contains("container/beta.container"));

    assert_eq!(
        json["revision_context"]["desired_revision"].as_str(),
        Some("rev-2")
    );
    assert_eq!(
        json["revision_context"]["desired_requested_ref"].as_str(),
        Some("demo-uat-v2")
    );
    assert_eq!(json["summary"]["changed_count"].as_u64(), Some(1));
    assert_eq!(json["summary"]["failed_count"].as_u64(), Some(1));
    assert_eq!(json["outcome"].as_str(), Some("non_converging"));
    assert_eq!(
        json["events"][1]["object"]["display_id"].as_str(),
        Some("container/beta.container")
    );
    assert_eq!(json["events"][1]["state"].as_str(), Some("failed"));
}
