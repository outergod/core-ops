use core_ops::cli::report::format_verification_run_report;
use core_ops::cli::verification::{execute_scenario, run, VerificationExecutionContext, VerifyRunArgs};
use core_ops::core::types::{
    VerificationArtifactCollectionStatus, VerificationRunMode, VerificationRunOutcome,
};
use core_ops::core::verification_model::{
    load_scenario_definition, VerificationAssertionSpec, VerificationCoreOpsAction,
    VerificationCoreOpsActionKind, VerificationScenarioClass, VerificationScenarioStep,
    VerificationStepTarget, VerificationStepType,
};
use core_ops::io::guest::GuestCommandRunner;
use core_ops::io::libvirt::LibvirtCommandRunner;
use core_ops::io::verification_artifacts::ArtifactCollector;
use std::fs;

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn accepted_scenario_execution_passes_and_collects_artifacts() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };

    let view = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-accepted",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Passed);
    assert_eq!(
        view.artifact_bundle.collection_status,
        VerificationArtifactCollectionStatus::Complete
    );
    assert!(!view.environment_retained);
}

#[test]
fn failed_assertion_classifies_as_assertion_failure() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    scenario.assertions[0].assertion_type = "output_contains".to_string();
    scenario.assertions[0].expected_state = "missing-output".to_string();

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let view = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-failed-assertion",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(
        view.overall_outcome,
        VerificationRunOutcome::AssertionFailure
    );
    assert!(view
        .failure_summary
        .as_deref()
        .unwrap_or("")
        .contains("assertions"));
}

#[test]
fn partial_artifact_collection_preserves_primary_outcome() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let mut overrides = scenario.policy_overrides.clone().unwrap_or(
        core_ops::core::verification_model::VerificationHarnessPolicyOverride {
            timeout_profile: None,
            timeouts: None,
            artifact_profile: None,
            artifact_policy: None,
        },
    );
    let mut artifact_policy = scenario
        .effective_artifact_policy()
        .expect("artifact policy");
    artifact_policy
        .always_collect
        .push("force-fail-artifact".to_string());
    overrides.artifact_policy = Some(artifact_policy);
    scenario.policy_overrides = Some(overrides);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let view = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-partial-artifacts",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Passed);
    assert_eq!(
        view.artifact_bundle.collection_status,
        VerificationArtifactCollectionStatus::Partial
    );
    assert!(!view.warnings.is_empty());
}

#[test]
fn repeated_execution_preserves_meaningful_outcome_shape() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;

    let first_workspace = workspace.path().join("run-one");
    let first_context = VerificationExecutionContext {
        workspace: &first_workspace,
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let first = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-one",
        &first_context,
        false,
    )
    .expect("first execute");

    let second_workspace = workspace.path().join("run-two");
    let second_context = VerificationExecutionContext {
        workspace: &second_workspace,
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let second = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-two",
        &second_context,
        false,
    )
    .expect("second execute");

    assert_eq!(first.overall_outcome, VerificationRunOutcome::Passed);
    assert_eq!(second.overall_outcome, VerificationRunOutcome::Passed);
    assert_eq!(first.scenario_id, second.scenario_id);
    assert_eq!(first.revision_under_test, second.revision_under_test);
    assert_eq!(first.step_results, second.step_results);
    assert_eq!(first.assertion_results, second.assertion_results);
    assert_eq!(first.failure_summary, second.failure_summary);
    assert_eq!(first.environment_retained, second.environment_retained);
    assert_eq!(
        first.artifact_bundle.collection_status,
        second.artifact_bundle.collection_status
    );
    assert_ne!(first.run_id, second.run_id);
    assert_ne!(first.artifact_bundle.bundle_path, second.artifact_bundle.bundle_path);
}

#[test]
fn ci_corpus_exit_code_is_non_zero_when_any_accepted_scenario_fails() {
    let corpus = tempfile::tempdir().expect("corpus");
    let passing = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("passing scenario");
    let mut failing = passing.clone();
    failing.scenario_id = "verify-idempotent-frontend-failing".to_string();
    failing.title = "Frontend idempotency fails".to_string();
    failing.assertions[0].assertion_type = "output_contains".to_string();
    failing.assertions[0].expected_state = "missing-output".to_string();
    write_scenario_fixture(corpus.path().join("passing.yaml"), &passing);
    write_scenario_fixture(corpus.path().join("failing.yaml"), &failing);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = run(&VerifyRunArgs {
        scenario: None,
        accepted_dir: Some(corpus.path().to_path_buf()),
        scenario_ids: Vec::new(),
        workspace_root: Some(workspace.path().to_path_buf()),
        artifacts_dir: Some(artifacts.path().to_path_buf()),
        debug: false,
        synthetic: true,
        ci: true,
        json: true,
        verbose: false,
    })
    .expect("run corpus");

    assert_eq!(output.exit_code, 1);
    let parsed: serde_json::Value =
        serde_json::from_str(&output.machine_report).expect("valid suite json");
    assert_eq!(parsed["mode"], "ci");
    assert_eq!(parsed["overall_outcome"], "assertion_failure");
    assert_eq!(
        parsed["scenario_outcomes"].as_array().expect("array").len(),
        2
    );
}

#[test]
fn ci_corpus_reporting_preserves_revision_provenance_for_repository_evolution() {
    let corpus = tempfile::tempdir().expect("corpus");
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    write_scenario_fixture(corpus.path().join("accepted.yaml"), &scenario);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = run(&VerifyRunArgs {
        scenario: None,
        accepted_dir: Some(corpus.path().to_path_buf()),
        scenario_ids: Vec::new(),
        workspace_root: Some(workspace.path().to_path_buf()),
        artifacts_dir: Some(artifacts.path().to_path_buf()),
        debug: false,
        synthetic: true,
        ci: true,
        json: true,
        verbose: false,
    })
    .expect("run corpus");

    assert_eq!(output.exit_code, 0);
    let parsed: serde_json::Value =
        serde_json::from_str(&output.machine_report).expect("valid suite json");
    assert_eq!(parsed["revision_selection_basis"], "accepted_corpus");
    assert_eq!(parsed["revision_under_test"], "demo-uat-v2");
    assert_eq!(
        parsed["scenario_outcomes"][0]["revision_under_test"],
        "demo-uat-v2"
    );

    let bundle_path = parsed["artifacts"]["bundle_path"]
        .as_str()
        .expect("bundle path");
    let bundle_index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(std::path::Path::new(bundle_path).join("scenario-bundles.json"))
            .expect("read bundle index"),
    )
    .expect("valid bundle index");
    assert_eq!(
        bundle_index["scenario_bundles"][0]["scenario_id"],
        "verify-idempotent-frontend"
    );
    assert_eq!(
        bundle_index["scenario_bundles"][0]["revision_under_test"],
        "demo-uat-v2"
    );
}

#[test]
fn ci_bug_reproduction_rerun_can_focus_single_accepted_regression_scenario() {
    let corpus = tempfile::tempdir().expect("corpus");
    let mut regression = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    regression.scenario_id = "verify-regression-frontend".to_string();
    regression.title = "Regression rerun remains fixed".to_string();
    regression.behavioral_claim = "A reproduced frontend regression remains fixed.".to_string();
    regression.rationale = "Permanent accepted regression scenario for a prior bug reproduction.".to_string();
    regression.scenario_classes = vec![VerificationScenarioClass::RegressionDetection];
    write_scenario_fixture(corpus.path().join("regression.yaml"), &regression);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = run(&VerifyRunArgs {
        scenario: None,
        accepted_dir: Some(corpus.path().to_path_buf()),
        scenario_ids: vec!["verify-regression-frontend".to_string()],
        workspace_root: Some(workspace.path().to_path_buf()),
        artifacts_dir: Some(artifacts.path().to_path_buf()),
        debug: false,
        synthetic: true,
        ci: true,
        json: true,
        verbose: false,
    })
    .expect("run regression rerun");

    assert_eq!(output.exit_code, 0);
    assert!(output.human_report.contains("verify-regression-frontend @ demo-uat-v2: passed"));
    let parsed: serde_json::Value =
        serde_json::from_str(&output.machine_report).expect("valid suite json");
    let outcomes = parsed["scenario_outcomes"].as_array().expect("array");
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["scenario_id"], "verify-regression-frontend");
    assert_eq!(outcomes[0]["revision_under_test"], "demo-uat-v2");
}

#[test]
fn command_surface_assertions_cover_machine_human_agent_and_timing_interfaces() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    scenario.scenario_id = "verify-command-surfaces".to_string();
    scenario.title = "Command surfaces remain stable".to_string();
    scenario.steps = vec![
        VerificationScenarioStep {
            step_id: "boot".to_string(),
            step_type: VerificationStepType::Boot,
            target: VerificationStepTarget::Guest,
            action: None,
            command: None,
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
        VerificationScenarioStep {
            step_id: "plan-json".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            target: VerificationStepTarget::Guest,
            action: Some(VerificationCoreOpsAction {
                action: VerificationCoreOpsActionKind::Plan,
                repository_source: "fixture".to_string(),
                revision: "demo-uat-v2".to_string(),
                object: None,
                host: None,
                mode: Some("json".to_string()),
                output_contract: Some("machine-readable".to_string()),
            }),
            command: None,
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
        VerificationScenarioStep {
            step_id: "apply-humane".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            target: VerificationStepTarget::Guest,
            action: Some(VerificationCoreOpsAction {
                action: VerificationCoreOpsActionKind::Apply,
                repository_source: "fixture".to_string(),
                revision: "demo-uat-v2".to_string(),
                object: None,
                host: None,
                mode: Some("humane".to_string()),
                output_contract: None,
            }),
            command: None,
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
        VerificationScenarioStep {
            step_id: "agent-run".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            target: VerificationStepTarget::Guest,
            action: Some(VerificationCoreOpsAction {
                action: VerificationCoreOpsActionKind::Agent,
                repository_source: "fixture".to_string(),
                revision: "demo-uat-v2".to_string(),
                object: None,
                host: None,
                mode: None,
                output_contract: None,
            }),
            command: None,
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
        VerificationScenarioStep {
            step_id: "status-human".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            target: VerificationStepTarget::Guest,
            action: Some(VerificationCoreOpsAction {
                action: VerificationCoreOpsActionKind::Status,
                repository_source: "fixture".to_string(),
                revision: "demo-uat-v2".to_string(),
                object: None,
                host: None,
                mode: None,
                output_contract: None,
            }),
            command: None,
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
    ];
    scenario.assertions = vec![
        build_assertion(
            "plan-json-flag",
            "step_command_contains",
            "plan-json",
            "--json",
            "plan should use machine-readable mode",
        ),
        build_assertion(
            "plan-json-output",
            "step_stdout_contains",
            "plan-json",
            "\"command\":\"plan\"",
            "plan should emit machine-readable output",
        ),
        build_assertion(
            "humane-has-no-json",
            "step_command_not_contains",
            "apply-humane",
            "--json",
            "human-readable apply should not force json",
        ),
        build_assertion(
            "humane-has-no-verbose",
            "step_command_not_contains",
            "apply-humane",
            "--verbose",
            "humane apply should not force verbose mode",
        ),
        build_assertion(
            "humane-output-shape",
            "step_stdout_contains",
            "apply-humane",
            "Outcome: converged",
            "human-readable apply should emit a humane summary",
        ),
        build_assertion(
            "agent-command",
            "step_command_contains",
            "agent-run",
            " core-ops agent",
            "agent interface should render the public agent command",
        ),
        build_assertion(
            "agent-output",
            "step_stdout_contains",
            "agent-run",
            "agent simulated",
            "agent run should surface agent output",
        ),
        build_assertion(
            "status-command",
            "step_command_contains",
            "status-human",
            "core-ops status",
            "status should render its human-readable surface",
        ),
        build_assertion(
            "timing-budget",
            "step_duration_within_ms",
            "plan-json",
            "1000",
            "plan json command should stay within the coarse timing budget",
        ),
    ];

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
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
        "run-command-surfaces",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Passed);
    assert!(view
        .step_results
        .iter()
        .all(|step| step.duration_ms.is_some()));
}

#[test]
fn scenario_timeout_enforcement_classifies_run_as_timeout() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let mut overrides = scenario.policy_overrides.clone().unwrap_or(
        core_ops::core::verification_model::VerificationHarnessPolicyOverride {
            timeout_profile: None,
            timeouts: None,
            artifact_profile: None,
            artifact_policy: None,
        },
    );
    let mut timeouts = scenario.effective_timeouts().expect("timeouts");
    timeouts.scenario_timeout = "0s".to_string();
    overrides.timeouts = Some(timeouts);
    scenario.policy_overrides = Some(overrides);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
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
        "run-scenario-timeout",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Timeout);
    assert!(view
        .step_results
        .iter()
        .any(|step| step.status == core_ops::core::types::VerificationStepStatus::TimedOut));
}

#[test]
fn failing_regression_scenario_enriches_artifacts_and_report_surfaces() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    scenario.scenario_id = "verify-regression-failure".to_string();
    scenario.title = "Regression failure is diagnosable".to_string();
    scenario.behavioral_claim = "A promoted regression remains diagnosable when it fails.".to_string();
    scenario.rationale = "Accepted bug reproductions should retain comparison and promotion context.".to_string();
    scenario.scenario_classes = vec![VerificationScenarioClass::RegressionDetection];
    scenario.assertions[0].assertion_type = "output_contains".to_string();
    scenario.assertions[0].expected_state = "missing-output".to_string();

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = GuestCommandRunner::default();
    let collector = ArtifactCollector;
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
        "run-regression-failure",
        &context,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::AssertionFailure);
    assert!(view
        .regression_summary
        .as_deref()
        .unwrap_or("")
        .contains("Revision sequence: demo-uat-v1 -> demo-uat-v2"));
    assert_eq!(
        view.promotion_status.as_deref(),
        Some("accepted permanent regression scenario derived from a bug reproduction")
    );
    assert!(view
        .artifact_bundle
        .failure_specific_entries
        .contains(&"failure-summary".to_string()));
    assert!(view
        .artifact_bundle
        .failure_specific_entries
        .contains(&"regression-summary".to_string()));

    let bundle_path = std::path::Path::new(&view.artifact_bundle.bundle_path);
    assert!(bundle_path.join("failure-summary.txt").exists());
    assert!(bundle_path.join("regression-summary.txt").exists());
    assert!(bundle_path.join("promotion-status.txt").exists());

    let report = format_verification_run_report(&view);
    assert!(report.contains("Regression: demo-uat-v1 -> demo-uat-v2"));
    assert!(report.contains("Promotion: accepted permanent regression scenario"));
    assert!(report.contains("Failure-Specific Artifacts:"));
}

fn write_scenario_fixture(path: std::path::PathBuf, scenario: &core_ops::core::verification_model::VerificationScenarioDefinition) {
    let contents = serde_yaml::to_string(scenario).expect("serialize scenario");
    fs::write(path, contents).expect("write scenario fixture");
}

fn build_assertion(
    assertion_id: &str,
    assertion_type: &str,
    target: &str,
    expected_state: &str,
    failure_message: &str,
) -> VerificationAssertionSpec {
    VerificationAssertionSpec {
        assertion_id: assertion_id.to_string(),
        assertion_type: assertion_type.to_string(),
        target: target.to_string(),
        expected_state: expected_state.to_string(),
        failure_message: failure_message.to_string(),
        artifact_hints: Vec::new(),
    }
}
