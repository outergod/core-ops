use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::cli::apply::{execute_rollback_with_report, rollback_with_report};
use core_ops::cli::status::render_rollback_summary;
use core_ops::core::reconcile::reconcile_rollback;
use core_ops::core::types::{
    DeterministicPersistedState, ManagedObjectKind, NormalizedManagedObject, NormalizedSnapshot,
    RetainedAppliedSnapshot, RollbackEligibility,
};
use core_ops::io::state::write_deterministic_state;
use crate::integration::env_lock::path_lock;

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
            .collect::<BTreeMap<_, _>>(),
        dependency_refs: dependency_refs.iter().map(|value| value.to_string()).collect(),
    }
}

fn temp_file(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}.json", prefix, nanos));
    path
}

fn write_systemctl_stub(dir: &PathBuf) {
    let bin_path = dir.join("systemctl");
    let script = "#!/bin/sh\nexit 0\n";
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
fn rollback_planning_and_execution_against_retained_successful_revision() {
    let target_snapshot = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "old")],
                &[],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "stable")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };
    let current_snapshot = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "new")],
                &[],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service"), ("image", "canary")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![
            RetainedAppliedSnapshot {
                revision_id: "rev-1".to_string(),
                scope_id: "host:alpha".to_string(),
                snapshot: target_snapshot.clone(),
                retained: true,
            },
            RetainedAppliedSnapshot {
                revision_id: "rev-2".to_string(),
                scope_id: "host:alpha".to_string(),
                snapshot: current_snapshot.clone(),
                retained: true,
            },
        ],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    let result = reconcile_rollback(&state, "host:alpha", "rev-1", &current_snapshot)
        .expect("rollback should plan");

    assert_eq!(result.target.eligibility, RollbackEligibility::Eligible);
    assert_eq!(result.plan.desired_revision_id.as_deref(), Some("rev-1"));
    assert_eq!(result.plan.baseline_revision_id.as_deref(), Some("rev-2"));
    assert!(result.summary.contains("rollback target=rev-1"));
    assert!(result
        .plan
        .actions
        .iter()
        .any(|action| action.reason.contains("rollback to rev-1")));

    let path = temp_file("deterministic_rollback_state");
    write_deterministic_state(&path, &state).expect("write deterministic state");
    let report = rollback_with_report(&path, "rev-1", &current_snapshot)
        .expect("rollback report");
    assert!(report.contains("rollback target=rev-1"));
    assert!(render_rollback_summary(&state).contains("eligibility=none"));
}

#[test]
fn rollback_rejected_when_retained_snapshot_metadata_is_missing_or_expired() {
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![RetainedAppliedSnapshot {
            revision_id: "rev-1".to_string(),
            scope_id: "host:alpha".to_string(),
            snapshot: NormalizedSnapshot {
                revision_id: Some("rev-1".to_string()),
                scope_id: "host:alpha".to_string(),
                objects: vec![object(
                    "alpha.service",
                    ManagedObjectKind::GeneratedUnit,
                    &[("unit", "alpha.service")],
                    &[],
                )],
            },
            retained: false,
        }],
        latest_convergence: None,
        latest_rollback_target: None,
    };
    let actual = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: Vec::new(),
    };

    let err = reconcile_rollback(&state, "host:alpha", "rev-1", &actual)
        .expect_err("expired rollback target must fail");
    assert!(err.message.contains("Expired"));

    let missing = reconcile_rollback(&state, "host:alpha", "rev-does-not-exist", &actual)
        .expect_err("missing rollback target must fail");
    assert!(missing.message.contains("MissingSnapshot"));
}

#[test]
fn rollback_report_includes_target_context_and_embedded_plan_summary() {
    let target_snapshot = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "stable")],
            &[],
        )],
    };
    let current_snapshot = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "canary")],
            &[],
        )],
    };
    let state = DeterministicPersistedState {
        schema_version: 1,
        current_scope: "host:alpha".to_string(),
        retained_snapshots: vec![
            RetainedAppliedSnapshot {
                revision_id: "rev-1".to_string(),
                scope_id: "host:alpha".to_string(),
                snapshot: target_snapshot.clone(),
                retained: true,
            },
            RetainedAppliedSnapshot {
                revision_id: "rev-2".to_string(),
                scope_id: "host:alpha".to_string(),
                snapshot: current_snapshot.clone(),
                retained: true,
            },
        ],
        latest_convergence: None,
        latest_rollback_target: None,
    };

    let path = temp_file("deterministic_rollback_contract");
    write_deterministic_state(&path, &state).expect("write deterministic state");
    let report = rollback_with_report(&path, "rev-1", &current_snapshot).expect("rollback report");

    assert!(report.contains("rollback target=rev-1"));
    assert!(report.contains("eligibility=Eligible"));
    assert!(report.contains("deterministic plan scope=host:alpha"));
    assert!(report.contains("desired_revision=rev-1"));
    assert!(report.contains("baseline_revision=rev-2"));
}

#[test]
fn rollback_contract_document_matches_current_report_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let contract = fs::read_to_string(
        root.join("specs/006-deterministic-reconcile/contracts/rollback.md"),
    )
    .expect("read rollback contract");

    assert!(contract.contains("rollback target=<target_revision_id>"));
    assert!(contract.contains("eligibility=<eligibility>"));
    assert!(contract.contains("deterministic plan summary"));
    assert!(contract.contains("completed_actions"));
    assert!(contract.contains("failed_actions"));
}

#[test]
fn rollback_plan_only_executes_the_reachable_cli_helper() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_file("rollback_repo");
    let repo_dir = repo.with_extension("");
    std::fs::create_dir_all(&repo_dir).expect("repo dir");
    std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(&repo_dir)
        .output()
        .expect("git init");
    std::fs::create_dir_all(repo_dir.join("quadlets")).expect("quadlets dir");
    std::fs::write(
        repo_dir.join("quadlets/alpha.container"),
        "[Container]\nImage=alpine:3.19\n",
    )
    .expect("write rev1");
    for message in ["rev1", "rev2"] {
        if message == "rev2" {
            std::fs::write(
                repo_dir.join("quadlets/alpha.container"),
                "[Container]\nImage=alpine:3.20\n",
            )
            .expect("write rev2");
        }
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("add")
            .arg(".")
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
            .output()
            .expect("git commit");
    }
    let rev1 = String::from_utf8_lossy(
        &std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("rev-list")
            .arg("--max-parents=0")
            .arg("HEAD")
            .output()
            .expect("git rev-list")
            .stdout,
    )
    .trim()
    .to_string();
    let rev2 = String::from_utf8_lossy(
        &std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .expect("git rev-parse")
            .stdout,
    )
    .trim()
    .to_string();

    let temp = temp_file("rollback_cli_state");
    let state_dir = temp.with_extension("");
    std::fs::create_dir_all(&state_dir).expect("state dir");
    let status_path = state_dir.join("status.json");
    let deterministic_path = state_dir.join("deterministic-state.json");
    let quadlet_dir = state_dir.join("host_quadlets");
    std::fs::create_dir_all(&quadlet_dir).expect("host quadlets");
    write_deterministic_state(
        &deterministic_path,
        &DeterministicPersistedState {
            schema_version: 1,
            current_scope: "scope:default".to_string(),
            retained_snapshots: vec![
                RetainedAppliedSnapshot {
                    revision_id: rev1.clone(),
                    scope_id: "scope:default".to_string(),
                    snapshot: NormalizedSnapshot {
                        revision_id: Some(rev1.clone()),
                        scope_id: "scope:default".to_string(),
                        objects: vec![object(
                            "alpha.service",
                            ManagedObjectKind::QuadletResource,
                            &[("unit_name", "alpha.service"), ("contents", "3.19")],
                            &[],
                        )],
                    },
                    retained: true,
                },
                RetainedAppliedSnapshot {
                    revision_id: rev2.clone(),
                    scope_id: "scope:default".to_string(),
                    snapshot: NormalizedSnapshot {
                        revision_id: Some(rev2.clone()),
                        scope_id: "scope:default".to_string(),
                        objects: vec![object(
                            "alpha.service",
                            ManagedObjectKind::QuadletResource,
                            &[("unit_name", "alpha.service"), ("contents", "3.20")],
                            &[],
                        )],
                    },
                    retained: true,
                },
            ],
            latest_convergence: None,
            latest_rollback_target: None,
        },
    )
    .expect("write deterministic state");

    let (result, report, _plan) = execute_rollback_with_report(
        repo_dir.to_str().expect("repo path"),
        &rev1,
        &quadlet_dir,
        false,
        Some(status_path),
        true,
    )
    .expect("rollback plan only");

    assert_eq!(result.run.summary, format!("rollback plan ready for {}", rev1));
    assert!(report.contains("rollback target="));
    assert!(report.contains("desired_revision="));
}

#[test]
fn representative_rollback_plan_and_execution_complete_within_sc003_budget() {
    let _lock = path_lock().lock().expect("path lock");
    let repo = temp_file("rollback_timing_repo");
    let repo_dir = repo.with_extension("");
    fs::create_dir_all(&repo_dir).expect("repo dir");
    std::process::Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .arg(&repo_dir)
        .output()
        .expect("git init");
    fs::create_dir_all(repo_dir.join("quadlets")).expect("quadlets dir");
    fs::write(
        repo_dir.join("quadlets/alpha.container"),
        "[Container]\nImage=alpine:3.19\n",
    )
    .expect("write rev1");
    for message in ["rev1", "rev2"] {
        if message == "rev2" {
            fs::write(
                repo_dir.join("quadlets/alpha.container"),
                "[Container]\nImage=alpine:3.20\n",
            )
            .expect("write rev2");
        }
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("add")
            .arg(".")
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
            .output()
            .expect("git commit");
    }
    let rev1 = String::from_utf8_lossy(
        &std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("rev-list")
            .arg("--max-parents=0")
            .arg("HEAD")
            .output()
            .expect("git rev-list")
            .stdout,
    )
    .trim()
    .to_string();
    let rev2 = String::from_utf8_lossy(
        &std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .expect("git rev-parse")
            .stdout,
    )
    .trim()
    .to_string();

    let temp = temp_file("rollback_timing_state");
    let state_dir = temp.with_extension("");
    fs::create_dir_all(&state_dir).expect("state dir");
    write_systemctl_stub(&state_dir);
    let old_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", state_dir.display(), old_path));
    let _path_guard = PathGuard { previous: old_path };

    let status_path = state_dir.join("status.json");
    let deterministic_path = state_dir.join("deterministic-state.json");
    let quadlet_dir = state_dir.join("host_quadlets");
    fs::create_dir_all(&quadlet_dir).expect("host quadlets");
    write_deterministic_state(
        &deterministic_path,
        &DeterministicPersistedState {
            schema_version: 1,
            current_scope: "scope:default".to_string(),
            retained_snapshots: vec![
                RetainedAppliedSnapshot {
                    revision_id: rev1.clone(),
                    scope_id: "scope:default".to_string(),
                    snapshot: NormalizedSnapshot {
                        revision_id: Some(rev1.clone()),
                        scope_id: "scope:default".to_string(),
                        objects: vec![object(
                            "alpha.service",
                            ManagedObjectKind::QuadletResource,
                            &[("unit_name", "alpha.service"), ("contents", "3.19")],
                            &[],
                        )],
                    },
                    retained: true,
                },
                RetainedAppliedSnapshot {
                    revision_id: rev2.clone(),
                    scope_id: "scope:default".to_string(),
                    snapshot: NormalizedSnapshot {
                        revision_id: Some(rev2),
                        scope_id: "scope:default".to_string(),
                        objects: vec![object(
                            "alpha.service",
                            ManagedObjectKind::QuadletResource,
                            &[("unit_name", "alpha.service"), ("contents", "3.20")],
                            &[],
                        )],
                    },
                    retained: true,
                },
            ],
            latest_convergence: None,
            latest_rollback_target: None,
        },
    )
    .expect("write deterministic state");

    let started = Instant::now();
    let (_plan_result, plan_report, _plan) = execute_rollback_with_report(
        repo_dir.to_str().expect("repo path"),
        &rev1,
        &quadlet_dir,
        false,
        Some(status_path.clone()),
        true,
    )
    .expect("rollback plan only");
    let (_apply_result, apply_report, _plan) = execute_rollback_with_report(
        repo_dir.to_str().expect("repo path"),
        &rev1,
        &quadlet_dir,
        false,
        Some(status_path),
        false,
    )
    .expect("rollback execute");
    let elapsed = started.elapsed();

    assert!(plan_report.contains("rollback target="));
    assert!(apply_report.contains("rollback target="));
    assert!(
        elapsed.as_secs() < 300,
        "representative rollback exceeded SC-003 budget: {:?}",
        elapsed
    );
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}
