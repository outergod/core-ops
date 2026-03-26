use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::core::reconcile::reconcile_deterministic_plan;
use core_ops::core::types::{DriftCategory, ManagedObjectKind, NormalizedManagedObject, NormalizedSnapshot};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
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
    std::fs::create_dir_all(&quadlets).expect("create quadlets");
    std::fs::write(quadlets.join("alpha.container"), "[Container]\nImage=alpine")
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

#[test]
fn plan_does_not_apply_changes() {
    let repo = temp_dir("core_ops_repo_plan");
    let rev = init_git_repo(&repo);

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
    assert!(result.plan.actions.len() >= 1);
    assert!(std::fs::read_dir(&host_quadlets).unwrap().next().is_none());
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
        dependency_refs: dependency_refs.iter().map(|value| value.to_string()).collect(),
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
                &[("unit", "alpha.service"), ("image", "ghcr.io/example:debug")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");

    assert_eq!(result.plan.actions[0].object_id, "config:/etc/alpha/env");
    assert_eq!(result.plan.actions[1].object_id, "alpha.service");
    assert_eq!(result.plan.drift_records.len(), 1);
    assert_eq!(result.plan.drift_records[0].category, DriftCategory::ExternalDrift);
    assert!(result.plan.actions[1]
        .semantic_diff
        .get("image")
        .expect("image diff")
        .contains("ghcr.io/example:debug"));
}
