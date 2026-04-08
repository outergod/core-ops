use core_ops::cli::report::format_verification_run_report;
use core_ops::core::types::{
    VerificationArtifactCollectionStatus, VerificationRunMode, VerificationRunOutcome,
};
use core_ops::core::verification_model::{build_artifact_bundle, VerificationRunView};
use std::process::Command;

#[test]
fn core_ops_version_surface_exposes_release_identity() {
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops"))
        .arg("--version")
        .output()
        .expect("run core-ops --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn verification_report_human_surface_contains_controller_version() {
    let view = VerificationRunView {
        view_kind: "verification_run".to_string(),
        run_id: "run-distribution".to_string(),
        scenario_id: "distribution".to_string(),
        title: "Distribution".to_string(),
        mode: VerificationRunMode::Local,
        controller_version: env!("CARGO_PKG_VERSION").to_string(),
        revision_selection_basis: core_ops::core::verification_model::VerificationRevisionSelectionBasis::SingleScenario,
        revision_under_test: "demo".to_string(),
        overall_outcome: VerificationRunOutcome::Passed,
        started_at: "2026-04-08T00:00:00Z".to_string(),
        completed_at: "2026-04-08T00:00:01Z".to_string(),
        environment_retained: false,
        artifact_bundle: build_artifact_bundle(
            "/tmp/distribution",
            Vec::new(),
            false,
            VerificationArtifactCollectionStatus::Complete,
        ),
        step_results: Vec::new(),
        assertion_results: Vec::new(),
        warnings: Vec::new(),
        failure_summary: None,
        regression_summary: None,
        promotion_status: None,
        readiness_evidence: None,
    };

    let report = format_verification_run_report(&view);
    assert!(report.contains("Controller:"));
    assert!(report.contains(env!("CARGO_PKG_VERSION")));
}
