use std::collections::BTreeMap;

use core_ops::cli::plan::render_deterministic_plan;
use core_ops::cli::report::build_plan_output;
use core_ops::cli::report::format_deterministic_plan_report_with_options;
use core_ops::core::reconcile::{
    reconcile_deterministic_plan, reconcile_deterministic_plan_with_runtime,
};
use core_ops::core::types::{
    DeterministicActionClass, DriftCategory, ManagedObjectKind, NormalizedManagedObject,
    NormalizedSnapshot, PlanEntryAction, VerificationResult, VerificationStatus,
};

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
        dependency_refs: dependency_refs
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
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

#[test]
fn deterministic_three_way_planning_and_no_op_detection_covers_required_resource_kinds() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "abc123")],
                &[],
            ),
            object(
                "alpha.network",
                ManagedObjectKind::QuadletResource,
                &[("unit", "alpha-network.service")],
                &[],
            ),
            object(
                "var-lib-alpha.automount",
                ManagedObjectKind::Automount,
                &[("unit", "var-lib-alpha.automount")],
                &[],
            ),
            object(
                "var-lib-alpha.mount",
                ManagedObjectKind::Mount,
                &[("unit", "var-lib-alpha.mount")],
                &["var-lib-alpha.automount"],
            ),
            object(
                "alpha.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "alpha.service")],
                &[
                    "config:/etc/alpha/env",
                    "var-lib-alpha.mount",
                    "alpha.network",
                ],
            ),
        ],
    };
    let actual = desired.clone();
    let last_applied = desired.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);
    let machine = build_plan_output(&result.plan);

    assert_eq!(result.plan.actions.len(), 5);
    assert!(result
        .plan
        .actions
        .iter()
        .all(|action| action.classification == DeterministicActionClass::NoOp));
    assert_eq!(result.plan.drift_records.len(), 0);
    assert_eq!(
        result.plan.actions[0].object_id, "alpha.network",
        "dependency-free objects sort first deterministically"
    );
    assert!(summary.contains("Plan for host alpha @ rev-1"));
    assert!(!summary.contains("(first run)"));
    assert!(summary.contains("5 unchanged"));
    let mut lines = summary.lines();
    let header = lines.next().expect("header");
    let separator = lines.next().expect("separator");
    assert_eq!(separator.chars().count(), header.chars().count());
    assert!(summary.contains("alpha.service"));
    assert!(!summary.contains("requires"));
    assert_eq!(machine.entries.len(), 5);
    assert!(machine
        .entries
        .iter()
        .all(|entry| matches!(entry.action, PlanEntryAction::NoOp)));
    assert_eq!(
        machine.entries[4].object.display_id,
        "service/alpha.service"
    );
    assert_eq!(machine.entries[4].dependencies.len(), 3);
}

#[test]
fn deterministic_plan_verbose_mode_expands_unchanged_dependency_trees() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "abc123")],
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
    let rendered = format_deterministic_plan_report_with_options(
        &reconcile_deterministic_plan(&desired, Some(&desired), &desired)
            .expect("deterministic plan")
            .plan,
        true,
    );
    let summary = strip_ansi(&rendered);

    assert!(summary.contains("[·] Unchanged • 3"));
    assert!(summary.contains("requires"));
    assert!(summary.contains("config/etc/alpha/env"));
    assert!(summary.contains("network/alpha.network"));
}

#[test]
fn deterministic_plan_without_last_applied_does_not_invent_drift_for_converged_objects() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "config:/etc/alpha/env",
            ManagedObjectKind::RenderedArtifact,
            &[
                ("name", "/etc/alpha/env"),
                ("unit_name", "/etc/alpha/env"),
                ("quadlet_type", "configfile"),
                ("contents", "A=B"),
                ("enabled_state", "enabled"),
                ("restart_policy", "always"),
            ],
            &[],
        )],
    };
    let actual = desired.clone();

    let result = reconcile_deterministic_plan(&desired, None, &actual).expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);

    assert!(result.plan.drift_records.is_empty());
    assert!(result
        .plan
        .actions
        .iter()
        .all(|action| action.semantic_diff.is_empty()));
    assert!(!summary.contains("(with drift)"));
    assert!(!summary.contains("Δ "));
    assert!(summary.contains("[·] Unchanged • 1"));
}

#[test]
fn deterministic_plan_header_shows_revision_transition_when_baseline_differs() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:2")],
            &[],
        )],
    };
    let last_applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:1")],
            &[],
        )],
    };
    let actual = last_applied.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);

    assert!(summary.contains("Plan for host alpha @ rev-1 → rev-2"));
    assert!(!summary.contains("(with drift)"));
}

#[test]
fn deterministic_plan_expected_deletion_does_not_mark_revision_transition_as_drift() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![],
    };
    let last_applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "whoami.container",
            ManagedObjectKind::QuadletResource,
            &[
                ("unit", "whoami.container"),
                ("image", "docker.io/traefik/whoami:v1"),
            ],
            &[],
        )],
    };
    let actual = last_applied.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);

    assert!(summary.contains("Plan for host alpha @ rev-1 → rev-2"));
    assert!(!summary.contains("(with drift)"));
    assert_eq!(result.plan.drift_records.len(), 1);
    assert_eq!(
        result.plan.drift_records[0].category,
        DriftCategory::ExpectedChange
    );
    assert!(summary.contains("container/whoami.container"));
    assert!(summary.contains("orphaned"));
}

#[test]
fn deterministic_three_way_planning_flags_external_drift_without_losing_stable_order() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "abc123")],
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
        revision_id: Some("obs-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/alpha/env",
                ManagedObjectKind::RenderedArtifact,
                &[("path", "/etc/alpha/env"), ("content_sha", "abc123")],
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
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);
    let machine = build_plan_output(&result.plan);

    assert_eq!(result.plan.actions[0].object_id, "config:/etc/alpha/env");
    assert_eq!(result.plan.actions[1].object_id, "alpha.service");
    assert_eq!(
        result.plan.actions[1].classification,
        DeterministicActionClass::Update
    );
    assert_eq!(result.plan.drift_records.len(), 1);
    assert_eq!(
        result.plan.drift_records[0].category,
        DriftCategory::ExternalDrift
    );
    assert_eq!(machine.summary.changed_count, 1);
    assert_eq!(machine.summary.unchanged_count, 1);
    assert!(summary.contains("Plan for host alpha @ rev-2 (with drift)"));
    assert!(summary.contains("Summary"));
    assert!(summary.contains("1 update"));
    assert!(summary.contains("Δ image"));
    let changed_lines = summary.lines().collect::<Vec<_>>();
    assert!(changed_lines
        .iter()
        .any(|line| line.contains("service/alpha.service") && line.contains("update")));
    assert!(!summary.contains("actual state diverged from desired snapshot"));
    assert!(!summary.contains("0 unchanged"));
    assert!(!summary.contains("blockeds"));
    assert!(result.summary.contains("desired_revision=rev-2"));
}

#[test]
fn deterministic_three_way_planning_marks_converged_objects_no_op_after_expected_change() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:2")],
            &[],
        )],
    };
    let last_applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:1")],
            &[],
        )],
    };
    let actual = desired.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");

    assert_eq!(result.plan.actions.len(), 1);
    assert_eq!(
        result.plan.actions[0].classification,
        DeterministicActionClass::NoOp
    );
    assert_eq!(result.plan.drift_records.len(), 1);
    assert_eq!(
        result.plan.drift_records[0].category,
        DriftCategory::ExpectedChange
    );
}

#[test]
fn machine_plan_output_uses_new_contract_and_omits_legacy_top_level_diffs() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:2")],
            &[],
        )],
    };
    let last_applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "alpha.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "alpha.service"), ("image", "ghcr.io/example:1")],
            &[],
        )],
    };
    let actual = last_applied.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let parsed: serde_json::Value =
        serde_json::from_str(&rendered.machine).expect("parse machine plan");

    assert_eq!(parsed["view_kind"].as_str(), Some("plan"));
    assert!(parsed["revision_context"].is_object());
    assert!(parsed["summary"].is_object());
    assert!(parsed["entries"].is_array());
    assert!(parsed.get("diffs").is_none());
}

#[test]
fn deterministic_plan_surfaces_recover_for_runtime_reconciliation() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![object(
            "app.service",
            ManagedObjectKind::GeneratedUnit,
            &[("unit", "app.service"), ("image", "ghcr.io/example:2")],
            &[],
        )],
    };
    let actual = desired.clone();
    let last_applied = desired.clone();
    let verification_results = vec![VerificationResult {
        target: "app.service".to_string(),
        status: VerificationStatus::Failure,
        details: Some("unit not active: failed".to_string()),
    }];

    let result = reconcile_deterministic_plan_with_runtime(
        &desired,
        Some(&last_applied),
        &actual,
        &verification_results,
    )
    .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);
    let machine = build_plan_output(&result.plan);

    assert_eq!(
        result.plan.actions[0].classification,
        DeterministicActionClass::Recover
    );
    assert!(summary.contains("[↺] Recover • 1"));
    assert!(summary.contains("service/app.service"));
    assert!(summary.contains("recover"));
    assert_eq!(machine.summary.changed_count, 1);
    assert_eq!(machine.entries[0].action, PlanEntryAction::Recover);
    assert_eq!(
        machine.entries[0].causes[0].kind,
        core_ops::core::types::CauseKind::RuntimeVariance
    );
    assert!(machine.entries[0].diff.is_none());
}

#[test]
fn deterministic_plan_hides_historical_diffs_when_actual_already_matches_desired() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/caddy/Caddyfile",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "demo-v2")],
                &[],
            ),
            object(
                "frontend.container",
                ManagedObjectKind::QuadletResource,
                &[("contents", "frontend-v2")],
                &["config:/etc/caddy/Caddyfile"],
            ),
        ],
    };
    let last_applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/caddy/Caddyfile",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "demo-v1")],
                &[],
            ),
            object(
                "frontend.container",
                ManagedObjectKind::QuadletResource,
                &[("contents", "frontend-v1")],
                &["config:/etc/caddy/Caddyfile"],
            ),
        ],
    };
    let actual = desired.clone();
    let verification_results = vec![VerificationResult {
        target: "frontend.container".to_string(),
        status: VerificationStatus::Failure,
        details: Some("unit not active: failed".to_string()),
    }];

    let result = reconcile_deterministic_plan_with_runtime(
        &desired,
        Some(&last_applied),
        &actual,
        &verification_results,
    )
    .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);
    let machine = build_plan_output(&result.plan);

    let config_entry = machine
        .entries
        .iter()
        .find(|entry| entry.object.display_id == "config/etc/caddy/Caddyfile")
        .expect("config entry");
    let frontend_entry = machine
        .entries
        .iter()
        .find(|entry| entry.object.display_id == "container/frontend.container")
        .expect("frontend entry");

    assert_eq!(config_entry.action, PlanEntryAction::NoOp);
    assert_eq!(frontend_entry.action, PlanEntryAction::Recover);
    assert!(config_entry.diff.is_none());
    assert!(frontend_entry.diff.is_none());
    assert!(!summary.contains("Δ content"));
}

#[test]
fn deterministic_plan_renders_restart_with_because_and_dependency_tree() {
    let desired = NormalizedSnapshot {
        revision_id: Some("rev-2".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/app.env",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "DB_HOST=new")],
                &[],
            ),
            object(
                "data.mount",
                ManagedObjectKind::Mount,
                &[("unit", "data.mount")],
                &[],
            ),
            object(
                "app.container",
                ManagedObjectKind::QuadletResource,
                &[("unit", "app.container"), ("image", "stable")],
                &["config:/etc/app.env", "data.mount"],
            ),
            object(
                "app.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.service")],
                &["app.container"],
            ),
        ],
    };
    let applied = NormalizedSnapshot {
        revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        objects: vec![
            object(
                "config:/etc/app.env",
                ManagedObjectKind::RenderedArtifact,
                &[("contents", "DB_HOST=old")],
                &[],
            ),
            object(
                "data.mount",
                ManagedObjectKind::Mount,
                &[("unit", "data.mount")],
                &[],
            ),
            object(
                "app.container",
                ManagedObjectKind::QuadletResource,
                &[("unit", "app.container"), ("image", "stable")],
                &["config:/etc/app.env", "data.mount"],
            ),
            object(
                "app.service",
                ManagedObjectKind::GeneratedUnit,
                &[("unit", "app.service")],
                &["app.container"],
            ),
        ],
    };
    let actual = applied.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);
    let summary = strip_ansi(&rendered.summary);
    let machine = build_plan_output(&result.plan);

    let service_entry = machine
        .entries
        .iter()
        .find(|entry| entry.object.display_id == "service/app.service")
        .expect("service entry");
    let container_entry = machine
        .entries
        .iter()
        .find(|entry| entry.object.display_id == "container/app.container")
        .expect("container entry");

    assert_eq!(container_entry.action, PlanEntryAction::Restart);
    assert_eq!(service_entry.action, PlanEntryAction::Restart);
    assert_eq!(
        service_entry.causes[0]
            .source_object
            .as_ref()
            .map(|object| object.display_id.as_str()),
        Some("container/app.container")
    );
    assert!(summary.contains("Δ content"));
    assert!(!summary.contains("6 fields changed"));
    assert!(summary.contains("service/app.service"));
    assert!(summary.contains("dependency changed"));
    assert!(summary.contains("dependency changed: container/app.container"));
    assert!(summary.contains("└─ [↻] container/app.container"));
    assert!(summary.contains("├─ [~] config/etc/app.env"));
    assert!(summary.contains("└─ [·] mount/data.mount"));
    assert!(summary.contains("\n\nSummary\n"));
    assert!(!summary.contains("object missing from actual state"));
    assert!(!summary.contains("actual object is outside desired snapshot"));
}
