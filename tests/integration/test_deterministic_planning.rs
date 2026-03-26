use std::collections::BTreeMap;

use core_ops::cli::plan::render_deterministic_plan;
use core_ops::core::reconcile::reconcile_deterministic_plan;
use core_ops::core::types::{
    DeterministicActionClass, DriftCategory, ManagedObjectKind, NormalizedManagedObject,
    NormalizedSnapshot,
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
        dependency_refs: dependency_refs.iter().map(|value| value.to_string()).collect(),
    }
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
                &["config:/etc/alpha/env", "var-lib-alpha.mount", "alpha.network"],
            ),
        ],
    };
    let actual = desired.clone();
    let last_applied = desired.clone();

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");
    let rendered = render_deterministic_plan(&result.plan);

    assert_eq!(result.plan.actions.len(), 5);
    assert!(result
        .plan
        .actions
        .iter()
        .all(|action| action.classification == DeterministicActionClass::NoOp));
    assert_eq!(result.plan.drift_records.len(), 0);
    assert_eq!(
        result.plan.actions[0].object_id,
        "alpha.network",
        "dependency-free objects sort first deterministically"
    );
    assert!(rendered.summary.contains("deterministic plan scope=host:alpha"));
    assert!(rendered.summary.contains("no_op: alpha.service"));
    assert!(rendered.summary.contains("dependencies: config:/etc/alpha/env, var-lib-alpha.mount, alpha.network"));
    assert!(rendered.machine.contains("\"object_id\":\"alpha.service\""));
    assert!(rendered.machine.contains("\"classification\":\"no_op\""));
    assert!(rendered.machine.contains("\"graph\""));
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
                &[("unit", "alpha.service"), ("image", "ghcr.io/example:debug")],
                &["config:/etc/alpha/env"],
            ),
        ],
    };

    let result = reconcile_deterministic_plan(&desired, Some(&last_applied), &actual)
        .expect("deterministic plan");

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
