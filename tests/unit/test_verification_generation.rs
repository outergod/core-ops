use core_ops::core::verification_generate::{
    build_coverage_report, detect_duplicate_candidate, extract_verification_inputs,
    generate_candidates_from_spec, normalize_behavioral_claim,
};
use core_ops::core::verification_model::load_scenario_definition;

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn normalization_collapses_case_and_spacing() {
    assert_eq!(
        normalize_behavioral_claim("  Reapplying   the SAME revision  "),
        "reapplying the same revision"
    );
}

#[test]
fn duplicate_detection_rejects_matching_behavioral_claim_and_class() {
    let accepted = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("accepted scenario");
    let candidates = generate_candidates_from_spec(sample_spec(), std::slice::from_ref(&accepted))
        .expect("candidates");

    assert_eq!(candidates.len(), 1);
    assert!(detect_duplicate_candidate(
        &candidates[0].proposed_definition,
        &[accepted]
    ));
    assert_eq!(
        candidates[0].review_status,
        core_ops::core::verification_model::VerificationCandidateReviewStatus::Rejected
    );
}

#[test]
fn extraction_falls_back_to_feature_semantics_without_guidance_section() {
    let inputs = extract_verification_inputs(
        "# Feature Specification\n\n## Functional Requirements\n- The system SHALL reapply the same revision without reporting managed changes.\n",
    )
    .expect("derived inputs");

    assert!(!inputs.observable_behaviors.is_empty());
    assert!(inputs
        .required_scenario_classes
        .contains(&core_ops::core::verification_model::VerificationScenarioClass::Idempotency));
}

#[test]
fn coverage_report_identifies_missing_required_classes() {
    let accepted = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("accepted scenario");
    let report = build_coverage_report(sample_spec(), &[accepted]).expect("coverage report");

    assert!(report
        .missing_classes
        .contains(&core_ops::core::verification_model::VerificationScenarioClass::UpgradeTransition));
}

fn sample_spec() -> &'static str {
    r#"
# Feature Specification: Sample

## Verification Guidance

### Observable Behaviors

- Reapplying the same revision produces no managed changes
- Applying an upgraded revision preserves deterministic transition summaries

### Invariants

- Revision continuity remains visible

### Idempotency Expectations

- Reapplying the same revision remains stable

### Failure Modes

- Assertion failures remain diagnosable

### Upgrade Considerations

- Revision transitions remain explainable

### Required Scenario Classes

- idempotency
- upgrade_transition
"#
}
