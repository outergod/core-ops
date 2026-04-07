use core_ops::cli::report::format_verification_run_json;
use core_ops::cli::verification::{execute_scenario, VerificationExecutionContext};
use core_ops::core::types::{
    VerificationArtifactCollectionStatus, VerificationRunMode, VerificationRunOutcome,
};
use core_ops::core::verification_model::{
    build_artifact_bundle, load_scenario_definition, VerificationArtifactBundle,
    VerificationArtifactManifestEntry, VerificationReadinessEvidence, VerificationReadinessRecord,
    VerificationRevisionSelectionBasis, VerificationRun,
};
use core_ops::io::guest::GuestCommandRunner;
use core_ops::io::libvirt::LibvirtCommandRunner;
use core_ops::io::verification_artifacts::{build_run_artifacts, write_artifact_manifest};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn scenario_fixture_matches_foundational_contract() {
    let accepted = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("accepted scenario");
    let candidate = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-candidate.yaml",
    ))
    .expect("candidate scenario");

    assert_eq!(accepted.environment.profile, "single-blessed-vm");
    assert_eq!(accepted.fixtures.revision_under_test, "demo-uat-v2");
    assert_eq!(candidate.fixtures.revision_under_test, "demo-uat-v3");
    assert!(accepted
        .steps
        .iter()
        .any(|step| step.step_type == core_ops::core::verification_model::VerificationStepType::CoreopsAction));
}

#[test]
fn run_result_serialization_matches_contract_shape() {
    let fixture = std::fs::read_to_string(fixture_path(
        "tests/fixtures/verification/artifacts/run-result-passed.json",
    ))
    .expect("read run-result fixture");
    let parsed: serde_json::Value = serde_json::from_str(&fixture).expect("valid json");

    assert_eq!(parsed["view_kind"], "verification_run");
    assert_eq!(parsed["overall_outcome"], "passed");
    assert_eq!(parsed["mode"], "ci");

    let run = VerificationRun {
        run_id: "run-20260401-120001-frontend-idempotency".to_string(),
        mode: VerificationRunMode::Ci,
        revision_selection_basis: VerificationRevisionSelectionBasis::SingleScenario,
        revision_under_test: "demo-uat-v2".to_string(),
        controller_version: env!("CARGO_PKG_VERSION").to_string(),
        scenario_refs: vec!["verify-idempotent-frontend".to_string()],
        workspace_path: "artifacts/workspaces/run-1".to_string(),
        started_at: "2026-04-01T12:00:01Z".to_string(),
        completed_at: "2026-04-01T12:07:14Z".to_string(),
        overall_outcome: VerificationRunOutcome::Passed,
        artifact_bundle: build_artifact_bundle(
            "artifacts/run-20260401-120001-frontend-idempotency",
            vec![VerificationArtifactManifestEntry {
                logical_name: "scenario-definition".to_string(),
                relative_path: "scenario.yaml".to_string(),
                required: true,
            }],
            false,
            VerificationArtifactCollectionStatus::Complete,
        ),
    };
    let serialized = serde_json::to_value(&run).expect("serialize run");

    assert_eq!(serialized["run_id"], parsed["run_id"]);
    assert_eq!(serialized["overall_outcome"], parsed["overall_outcome"]);
    assert_eq!(
        serialized["revision_selection_basis"],
        parsed["revision_selection_basis"]
    );
}

#[test]
fn verification_run_json_output_matches_contract_shape() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = core_ops::io::verification_artifacts::ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };

    let view = execute_scenario(
        &scenario,
        VerificationRunMode::Ci,
        "run-20260401-120001-frontend-idempotency",
        &context,
        false,
        false,
    )
    .expect("execute");
    let parsed: serde_json::Value =
        serde_json::from_str(&format_verification_run_json(&view)).expect("json");

    assert_eq!(parsed["view_kind"], "verification_run");
    assert_eq!(parsed["mode"], "ci");
    assert_eq!(parsed["controller_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(parsed["revision_selection_basis"], "single_scenario");
    assert_eq!(parsed["revision_under_test"], "demo-uat-v2");
    assert_eq!(parsed["overall_outcome"], "passed");
    let started_at = parsed["started_at"].as_str().expect("started_at");
    let completed_at = parsed["completed_at"].as_str().expect("completed_at");
    assert!(started_at.ends_with('Z'));
    assert!(completed_at.ends_with('Z'));
    assert_ne!(started_at, "2026-04-01T00:00:00Z");
    assert_ne!(completed_at, "2026-04-01T00:00:01Z");
    assert_eq!(
        parsed["scenario_outcomes"][0]["scenario_id"],
        "verify-idempotent-frontend"
    );
    assert_eq!(
        parsed["scenario_outcomes"][0]["revision_under_test"],
        "demo-uat-v2"
    );
    assert_eq!(parsed["scenario_outcomes"][0]["outcome"], "passed");
    assert_eq!(
        parsed["scenario_outcomes"][0]["readiness_evidence"]["source"],
        "synthetic"
    );
    let observed = parsed["scenario_outcomes"][0]["assertion_results"][0]["observed_value"]
        .as_str()
        .expect("observed_value");
    assert!(!observed.contains('\u{1b}'));
    assert!(parsed["artifacts"]["bundle_path"].as_str().is_some());
    assert_eq!(parsed["artifacts"]["environment_retained"], false);
}

#[test]
fn artifact_manifest_can_be_written_for_foundational_bundle() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle = VerificationArtifactBundle {
        bundle_path: temp.path().join("bundle").display().to_string(),
        manifest_entries: vec![VerificationArtifactManifestEntry {
            logical_name: "scenario-definition".to_string(),
            relative_path: "scenario.yaml".to_string(),
            required: true,
        }],
        always_collected_entries: vec!["scenario-definition".to_string()],
        failure_specific_entries: Vec::new(),
        environment_retained: false,
        collection_status: VerificationArtifactCollectionStatus::Complete,
    };

    write_artifact_manifest(&bundle).expect("write manifest");
    let manifest_path = temp.path().join("bundle/manifest.json");
    assert!(manifest_path.exists());

    let artifacts = build_run_artifacts(
        temp.path().join("bundle-2").display().to_string(),
        vec![VerificationArtifactManifestEntry {
            logical_name: "console-log".to_string(),
            relative_path: "console.txt".to_string(),
            required: true,
        }],
        false,
    );
    assert_eq!(
        artifacts.bundle.always_collected_entries,
        vec!["console-log"]
    );
}

#[test]
fn readiness_evidence_serialization_matches_contract_shape() {
    let evidence = VerificationReadinessEvidence {
        source: "serial-console".to_string(),
        accepted_record: Some(VerificationReadinessRecord {
            run_id: "run-123".to_string(),
            token: "token-123".to_string(),
            ip: "192.0.2.10".to_string(),
            hostname: Some("vm-1".to_string()),
            ts: Some("2026-04-07T00:00:00Z".to_string()),
        }),
        rejected_records: Vec::new(),
        final_status: "accepted".to_string(),
        failure_summary: None,
    };

    let json = serde_json::to_value(&evidence).expect("serialize evidence");
    assert_eq!(json["source"], "serial-console");
    assert_eq!(json["accepted_record"]["run_id"], "run-123");
    assert_eq!(json["accepted_record"]["token"], "token-123");
    assert_eq!(json["accepted_record"]["ip"], "192.0.2.10");
    assert_eq!(json["final_status"], "accepted");
}
