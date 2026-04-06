use core_ops::core::verification_generate::{
    build_coverage_report, generate_candidates_from_spec, load_accepted_corpus,
    render_candidate_yaml,
};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn advisory_candidate_generation_produces_reviewable_candidate() {
    let accepted = load_accepted_corpus(&fixture_path("tests/fixtures/verification/scenarios"))
        .expect("accepted corpus");
    let spec = r#"
# Feature Specification: New Feature

## Verification Guidance

### Observable Behaviors

- Applying an upgraded revision preserves deterministic transition summaries

### Invariants

- Transition summaries stay stable

### Idempotency Expectations

- Reapplying the upgraded revision does not add new changes

### Failure Modes

- Upgrade failures remain diagnosable

### Upgrade Considerations

- Prior and target revision context remain visible

### Required Scenario Classes

- upgrade_transition
"#;

    let generated = generate_candidates_from_spec(spec, &accepted).expect("generated");
    assert_eq!(generated.len(), 1);
    assert_eq!(
        generated[0].review_status,
        core_ops::core::verification_model::VerificationCandidateReviewStatus::NeedsReview
    );
    assert_eq!(generated[0].proposed_definition.environment.profile, "single-blessed-vm");

    let yaml = render_candidate_yaml(&generated[0]).expect("yaml");
    assert!(yaml.contains("review_status: needs_review"));
    assert!(yaml.contains("behavioral_claim:"));
    assert!(yaml.contains("step_type: coreops_action"));

    let coverage = build_coverage_report(spec, &accepted).expect("coverage");
    assert!(coverage
        .missing_classes
        .contains(&core_ops::core::verification_model::VerificationScenarioClass::UpgradeTransition));
}

#[test]
fn generation_rejects_duplicate_accepted_coverage() {
    let accepted = load_accepted_corpus(&fixture_path("tests/fixtures/verification/scenarios"))
        .expect("accepted corpus");
    let spec = r#"
# Feature Specification: Duplicate

## Verification Guidance

### Observable Behaviors

- Reapplying the same revision produces no managed changes

### Invariants

- Idempotent applies remain stable

### Idempotency Expectations

- Reapplying the same revision remains stable

### Failure Modes

- Assertion failures remain diagnosable

### Upgrade Considerations

- Revision continuity remains visible

### Required Scenario Classes

- idempotency
"#;

    let generated = generate_candidates_from_spec(spec, &accepted).expect("generated");
    assert_eq!(
        generated[0].review_status,
        core_ops::core::verification_model::VerificationCandidateReviewStatus::Rejected
    );
    assert!(generated[0]
        .validation_findings
        .iter()
        .any(|finding| finding.contains("duplicate")));
}

#[test]
fn coverage_reporting_identifies_missing_required_scenario_classes() {
    let accepted = load_accepted_corpus(&fixture_path("tests/fixtures/verification/scenarios"))
        .expect("accepted corpus");
    let spec = r#"
# Feature Specification: Coverage

## Verification Guidance

### Observable Behaviors

- Reapplying the same revision produces no managed changes
- Applying an upgraded revision preserves deterministic transition summaries

### Invariants

- Transition summaries stay stable

### Idempotency Expectations

- Reapplying the same revision remains stable

### Failure Modes

- Assertion failures remain diagnosable

### Upgrade Considerations

- Revision transitions remain explainable

### Required Scenario Classes

- idempotency
- upgrade_transition
"#;

    let report = build_coverage_report(spec, &accepted).expect("coverage report");
    assert!(report
        .covered_classes
        .contains(&core_ops::core::verification_model::VerificationScenarioClass::Idempotency));
    assert!(report
        .missing_classes
        .contains(&core_ops::core::verification_model::VerificationScenarioClass::UpgradeTransition));
}
