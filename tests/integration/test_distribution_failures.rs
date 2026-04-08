use core_ops::cli::report::format_verification_run_report;
use core_ops::core::types::{
    VerificationArtifactCollectionStatus, VerificationRunMode, VerificationRunOutcome,
};
use core_ops::core::verification_model::{build_artifact_bundle, VerificationRunView};

#[test]
fn verification_failure_report_remains_actionable_and_versioned() {
    let view = VerificationRunView {
        view_kind: "verification_run".to_string(),
        run_id: "run-failure".to_string(),
        scenario_id: "distribution-failure".to_string(),
        title: "Distribution Failure".to_string(),
        mode: VerificationRunMode::Ci,
        controller_version: env!("CARGO_PKG_VERSION").to_string(),
        revision_selection_basis: core_ops::core::verification_model::VerificationRevisionSelectionBasis::SingleScenario,
        revision_under_test: "demo".to_string(),
        overall_outcome: VerificationRunOutcome::AssertionFailure,
        started_at: "2026-04-08T00:00:00Z".to_string(),
        completed_at: "2026-04-08T00:00:01Z".to_string(),
        environment_retained: false,
        artifact_bundle: build_artifact_bundle(
            "/tmp/distribution-failure",
            Vec::new(),
            false,
            VerificationArtifactCollectionStatus::Partial,
        ),
        step_results: Vec::new(),
        assertion_results: Vec::new(),
        warnings: Vec::new(),
        failure_summary: Some("release gate failed".to_string()),
        regression_summary: None,
        promotion_status: None,
        readiness_evidence: None,
    };

    let report = format_verification_run_report(&view);
    assert!(report.contains("Outcome: assertion_failure"));
    assert!(report.contains("Failure: release gate failed"));
    assert!(report.contains("Controller:"));
}
