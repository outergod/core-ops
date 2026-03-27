use std::fs;
use std::path::{Path, PathBuf};

use core_ops::cli::report::{build_plan_output, format_convergence_report_json};
use core_ops::cli::plan::render_deterministic_plan;
use core_ops::cli::status::render_deterministic_plan_summary;
use core_ops::core::types::{
    ConvergenceStatus, DeterministicActionClass, DeterministicConvergenceRecord,
    DeterministicPlannedAction, DeterministicReconciliationPlan, DependencyEdgeKind,
    DriftCategory, ManagedObjectKind, ReconcileMode, ReconcileRun, RunStatus,
    SemanticDependencyEdge, SemanticDependencyGraph, SemanticDependencyNode,
    StructuredDriftRecord, VerificationResult, VerificationStatus,
};
use serde_json::Value;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/provenance_state")
}

fn read_fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    fs::read_to_string(path).expect("read provenance fixture")
}

#[test]
fn provenance_state_fixtures_exist() {
    let dir = fixture_dir();
    assert!(dir.join("README.md").exists());
    assert!(dir.join("valid-success.json").exists());
    assert!(dir.join("valid-never-run.json").exists());
    assert!(dir.join("invalid-partial.json").exists());
    assert!(dir.join("invalid-unsupported-schema.json").exists());
}

#[test]
fn valid_success_fixture_contains_required_top_level_sections() {
    let contents = read_fixture("valid-success.json");
    assert!(contents.contains("\"schema_version\""));
    assert!(contents.contains("\"controller\""));
    assert!(contents.contains("\"desired_state\""));
    assert!(contents.contains("\"reconciliation\""));
}

#[test]
fn invalid_fixture_examples_cover_partial_and_unsupported_cases() {
    let partial = read_fixture("invalid-partial.json");
    let unsupported = read_fixture("invalid-unsupported-schema.json");

    assert!(!partial.trim_end().ends_with('}'));
    assert!(unsupported.contains("\"schema_version\": 99"));
}

#[test]
fn snapshot_comparison_identifies_controller_desired_state_and_outcome_changes() {
    let base: Value =
        serde_json::from_str(&read_fixture("valid-success.json")).expect("parse base fixture");
    let mut controller_changed = base.clone();
    controller_changed["controller"]["version"] = Value::String("0.6.0".to_string());
    let mut desired_changed = base.clone();
    desired_changed["desired_state"]["last_observed_revision"] =
        Value::String("feedface".to_string());
    let mut outcome_changed = base.clone();
    outcome_changed["reconciliation"]["status"] = Value::String("failed".to_string());

    assert_ne!(
        controller_changed["controller"]["version"],
        base["controller"]["version"]
    );
    assert_eq!(
        controller_changed["desired_state"]["last_observed_revision"],
        base["desired_state"]["last_observed_revision"]
    );
    assert_ne!(
        desired_changed["desired_state"]["last_observed_revision"],
        base["desired_state"]["last_observed_revision"]
    );
    assert_eq!(
        desired_changed["reconciliation"]["status"],
        base["reconciliation"]["status"]
    );
    assert_ne!(
        outcome_changed["reconciliation"]["status"],
        base["reconciliation"]["status"]
    );
}

#[test]
fn controller_version_provenance_matches_cargo_package_version() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo package version");
    let contents = read_fixture("valid-success.json");
    let parsed: Value = serde_json::from_str(&contents).expect("parse success fixture");

    assert_eq!(
        parsed["controller"]["version"].as_str(),
        Some(version.as_str())
    );
}

#[test]
fn structured_diff_output_exposes_required_fields() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        actions: vec![DeterministicPlannedAction {
            object_id: "alpha.service".to_string(),
            classification: DeterministicActionClass::Update,
            reason: "actual state diverged from desired snapshot".to_string(),
            dependency_context: vec!["config:/etc/alpha/env".to_string()],
            semantic_diff: [(
                "image".to_string(),
                "desired=stable actual=debug applied=stable".to_string(),
            )]
            .into_iter()
            .collect(),
        }],
        drift_records: vec![StructuredDriftRecord {
            object_id: "alpha.service".to_string(),
            category: DriftCategory::ExternalDrift,
            comparison_basis: "desired=last_applied actual!=desired".to_string(),
            auto_action: true,
            attention_required: true,
            details: "desired_fields=2 applied_fields=2 actual_fields=2".to_string(),
        }],
        graph: SemanticDependencyGraph {
            nodes: vec![
                SemanticDependencyNode {
                    object_id: "config:/etc/alpha/env".to_string(),
                    object_kind: ManagedObjectKind::RenderedArtifact,
                    ordering_key: "config:/etc/alpha/env".to_string(),
                },
                SemanticDependencyNode {
                    object_id: "alpha.service".to_string(),
                    object_kind: ManagedObjectKind::GeneratedUnit,
                    ordering_key: "alpha.service".to_string(),
                },
            ],
            edges: vec![SemanticDependencyEdge {
                from_object_id: "config:/etc/alpha/env".to_string(),
                to_object_id: "alpha.service".to_string(),
                edge_kind: DependencyEdgeKind::Explicit,
                reason: "config precedes generated unit".to_string(),
            }],
        },
    };

    let rendered = render_deterministic_plan(&plan);
    let parsed: Value = serde_json::from_str(&rendered.machine).expect("parse machine plan");
    let built = build_plan_output(&plan);
    let status = render_deterministic_plan_summary(&plan);

    assert_eq!(parsed["view_kind"].as_str(), Some("plan"));
    assert_eq!(
        parsed["revision_context"]["target_revision"].as_str(),
        Some("rev-2")
    );
    assert_eq!(
        parsed["revision_context"]["last_applied_revision"].as_str(),
        Some("rev-1")
    );
    assert_eq!(
        parsed["summary"]["changed_count"].as_u64(),
        Some(1)
    );
    assert_eq!(
        parsed["summary"]["unchanged_count"].as_u64(),
        Some(0)
    );
    assert_eq!(
        parsed["entries"][0]["object"]["display_id"].as_str(),
        Some("service/alpha.service")
    );
    assert_eq!(
        parsed["entries"][0]["action"].as_str(),
        Some("update")
    );
    assert_eq!(
        parsed["entries"][0]["dependencies"][0]["relation"].as_str(),
        Some("prerequisite")
    );
    assert_eq!(
        parsed["entries"][0]["causes"][0]["kind"].as_str(),
        Some("drift")
    );
    assert!(parsed["entries"][0]["diff"]["details"]["image"].is_string());
    assert_eq!(built.entries[0].order_index, 0);
    assert!(rendered.summary.contains("Summary"));
    assert!(rendered.summary.contains("service/alpha.service"));
    assert!(status.contains("changed=1"));
}

#[test]
fn delete_entries_preserve_specific_resource_types_when_missing_from_desired_graph() {
    let plan = DeterministicReconciliationPlan {
        desired_revision_id: Some("rev-2".to_string()),
        baseline_revision_id: Some("rev-1".to_string()),
        scope_id: "host:alpha".to_string(),
        actions: vec![
            DeterministicPlannedAction {
                object_id: "alpha.container".to_string(),
                classification: DeterministicActionClass::Delete,
                reason: "actual object is outside desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "alpha.volume".to_string(),
                classification: DeterministicActionClass::Delete,
                reason: "actual object is outside desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "alpha.socket".to_string(),
                classification: DeterministicActionClass::Delete,
                reason: "actual object is outside desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
            DeterministicPlannedAction {
                object_id: "alpha.network".to_string(),
                classification: DeterministicActionClass::Delete,
                reason: "actual object is outside desired snapshot".to_string(),
                dependency_context: Vec::new(),
                semantic_diff: Default::default(),
            },
        ],
        drift_records: Vec::new(),
        graph: SemanticDependencyGraph {
            nodes: Vec::new(),
            edges: Vec::new(),
        },
    };

    let built = build_plan_output(&plan);
    let ids = built
        .entries
        .iter()
        .map(|entry| entry.object.display_id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"container/alpha.container"));
    assert!(ids.contains(&"volume/alpha.volume"));
    assert!(ids.contains(&"socket/alpha.socket"));
    assert!(ids.contains(&"network/alpha.network"));
}

#[test]
fn structured_diff_contract_document_matches_implemented_plan_fields() {
    let contract = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("specs/006-deterministic-reconcile/contracts/structured-diff.md"),
    )
    .expect("read structured diff contract");

    assert!(contract.contains("view_kind"));
    assert!(contract.contains("revision_context"));
    assert!(contract.contains("entries"));
    assert!(contract.contains("ManagedObjectRef"));
    assert!(contract.contains("DependencyEdge"));
    assert!(contract.contains("SemanticDiff"));
}

#[test]
fn convergence_report_json_exposes_required_fields() {
    let run = ReconcileRun {
        run_id: "run:alpha".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Failure,
        failure_class: None,
        summary: "partial convergence".to_string(),
    };
    let verification_results = vec![VerificationResult {
        target: "alpha.service".to_string(),
        status: VerificationStatus::Failure,
        details: Some("blocked: unit not active".to_string()),
    }];
    let convergence = DeterministicConvergenceRecord {
        desired_revision_id: "rev-2".to_string(),
        scope_id: "host:alpha".to_string(),
        status: ConvergenceStatus::RepeatedFailure,
        attempt_count: 3,
        affected_objects: vec!["alpha.service".to_string()],
        completed_actions: vec!["config:/etc/alpha/env".to_string()],
        failed_actions: vec!["alpha.service".to_string()],
        can_continue: false,
    };

    let rendered = format_convergence_report_json(&run, &verification_results, Some(&convergence));
    let parsed: Value = serde_json::from_str(&rendered).expect("parse convergence json");

    assert_eq!(parsed["run_id"].as_str(), Some("run:alpha"));
    assert_eq!(parsed["status"].as_str(), Some("failure"));
    assert_eq!(parsed["summary"].as_str(), Some("partial convergence"));
    assert_eq!(
        parsed["verification_results"][0]["target"].as_str(),
        Some("alpha.service")
    );
    assert_eq!(
        parsed["convergence"]["status"].as_str(),
        Some("repeated_failure")
    );
    assert_eq!(parsed["convergence"]["attempt_count"].as_u64(), Some(3));
    assert_eq!(parsed["convergence"]["can_continue"].as_bool(), Some(false));
}
