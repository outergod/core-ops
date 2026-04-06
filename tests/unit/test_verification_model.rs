use core_ops::core::types::VerificationArtifactCollectionStatus;
use core_ops::core::verification_model::{
    build_artifact_bundle, load_scenario_definition, parse_scenario_definition,
};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn accepted_scenario_fixture_parses_and_validates() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("accepted scenario fixture");

    assert_eq!(scenario.scenario_id, "verify-idempotent-frontend");
    assert_eq!(scenario.environment.profile, "single-blessed-vm");
    assert_eq!(
        scenario
            .fixtures
            .repository_evolution
            .as_ref()
            .expect("repository evolution")
            .revisions,
        vec!["demo-uat-v1", "demo-uat-v2"]
    );
    assert_eq!(scenario.steps.len(), 3);
    assert_eq!(scenario.assertions.len(), 1);
}

#[test]
fn scenario_validation_rejects_missing_required_artifact_entries() {
    let raw = r#"
scenario_id: invalid-scenario
title: Invalid scenario
description: Missing required artifacts
scenario_classes: [idempotency]
source: accepted
behavioral_claim: Something stable happens.
rationale: Exercises validation.
environment:
  profile: single-blessed-vm
fixtures:
  repo_fixture: fixtures/repos/frontend
  revision_under_test: demo-uat-v2
steps:
  - step_id: boot
    step_type: boot
    target: guest
assertions:
  - assertion_id: stable
    assertion_type: no_pending_changes
    target: guest
    expected_state: none
    failure_message: failed
policy_overrides:
  artifact_policy:
    always_collect:
      - harness-log
      - console-log
      - coreops-output
      - assertion-results
    collect_on_failure:
      - explain-output
    retain_environment_in_debug: true
    export_format: directory
"#;

    let err = parse_scenario_definition(raw).expect_err("validation error");
    assert!(err.message.contains("scenario-definition"));
}

#[test]
fn artifact_bundle_marks_required_entries_as_always_collected() {
    let bundle = build_artifact_bundle(
        "artifacts/run-1",
        vec![
            core_ops::core::verification_model::VerificationArtifactManifestEntry {
                logical_name: "scenario-definition".to_string(),
                relative_path: "scenario.yaml".to_string(),
                required: true,
            },
            core_ops::core::verification_model::VerificationArtifactManifestEntry {
                logical_name: "journal-excerpts".to_string(),
                relative_path: "journal.txt".to_string(),
                required: false,
            },
        ],
        false,
        VerificationArtifactCollectionStatus::Partial,
    );

    assert_eq!(
        bundle.collection_status,
        VerificationArtifactCollectionStatus::Partial
    );
    assert_eq!(bundle.always_collected_entries, vec!["scenario-definition"]);
}
