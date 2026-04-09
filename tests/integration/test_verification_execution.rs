use core_ops::cli::report::{format_verification_run_json, format_verification_run_report};
use core_ops::cli::verification::{execute_scenario, run, VerificationExecutionContext, VerifyRunArgs};
use core_ops::core::types::{
    VerificationArtifactCollectionStatus, VerificationRunMode, VerificationRunOutcome,
};
use core_ops::core::verification_model::{
    load_scenario_definition, VerificationAssertionSpec, VerificationCoreOpsAction,
    VerificationCoreOpsActionKind, VerificationGuestReadinessPayload, VerificationScenarioClass,
    VerificationScenarioStep, VerificationStepTarget, VerificationStepType,
    VerificationReadinessAcquisition, VerificationReadinessEvidence, VerificationReadinessRejection,
    VerificationReadinessRejectionKind,
    VERIFICATION_READINESS_MARKER, VERIFICATION_READINESS_SCRIPT_PATH,
    VERIFICATION_READINESS_SERVICE_NAME,
};
use core_ops::core::boundaries::{VerificationGuestBoundary, VerificationLibvirtBoundary};
use core_ops::core::errors::CoreError;
use core_ops::core::types::FailureClass;
use core_ops::io::guest::GuestCommandRunner;
use core_ops::io::libvirt::LibvirtCommandRunner;
use core_ops::io::verification_artifacts::ArtifactCollector;
use std::fs;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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
        pause_before_teardown: false,
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
        pause_before_teardown: false,
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
        pause_before_teardown: false,
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

#[derive(Default)]
struct TimeoutGuestBoundary;

impl VerificationGuestBoundary for TimeoutGuestBoundary {
    fn wait_ready(
        &self,
        guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        timeout: &str,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Ok(core_ops::core::verification_model::GuestCommandOutput {
            status_code: 0,
            stdout: format!("{} ready within {timeout}", guest.guest_name),
            stderr: String::new(),
        })
    }

    fn run_command(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        _command: &str,
        _timeout: Option<&str>,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Err(CoreError::new(
            FailureClass::Transient,
            "ssh guest command timed out after 1s",
        ))
    }

    fn copy_to_guest(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        _local_path: &std::path::Path,
        _remote_path: &str,
        _recursive: bool,
        _executable: bool,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

#[test]
fn transient_guest_command_timeout_classifies_step_and_run_as_timeout() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = LibvirtCommandRunner::default();
    let guest = TimeoutGuestBoundary;
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
        "run-guest-timeout",
        &context,
        false,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Timeout);
    let timed_out_step = view
        .step_results
        .iter()
        .find(|step| step.step_id == "apply")
        .expect("apply step");
    assert_eq!(timed_out_step.status, core_ops::core::types::VerificationStepStatus::TimedOut);
    assert!(timed_out_step
        .details
        .as_deref()
        .unwrap_or("")
        .contains("timed out"));
}

struct SetupFailureLibvirtBoundary {
    destroy_calls: Arc<AtomicUsize>,
}

impl VerificationLibvirtBoundary for SetupFailureLibvirtBoundary {
    fn create_guest(
        &self,
        _scenario: &core_ops::core::verification_model::VerificationScenarioDefinition,
        workspace_root: &std::path::Path,
    ) -> Result<core_ops::core::verification_model::LibvirtGuestHandle, CoreError> {
        Ok(core_ops::core::verification_model::LibvirtGuestHandle {
            guest_name: "setup-failure-guest".to_string(),
            domain_name: "setup-failure-domain".to_string(),
            ssh_target: "core@192.0.2.10".to_string(),
            connection_uri: "qemu:///system".to_string(),
            workspace_root: workspace_root.display().to_string(),
            env_backed: true,
            network_mode: Some("dhcp".to_string()),
            vm_host: None,
            ssh_user: Some("core".to_string()),
            ignition_path: None,
            local_butane_path: None,
            local_ignition_path: None,
            volume_name: Some("setup-failure.qcow2".to_string()),
            assigned_ip: Some("192.0.2.10".to_string()),
            lease_path: None,
            rendered_network_config: None,
            serial_log_path: None,
            qemu_launch_log_path: None,
            readiness_payload: None,
            readiness_evidence: None,
        })
    }

    fn acquire_guest_readiness(
        &self,
        _scenario: &core_ops::core::verification_model::VerificationScenarioDefinition,
        guest: &core_ops::core::verification_model::LibvirtGuestHandle,
    ) -> Result<core_ops::core::verification_model::VerificationReadinessAcquisition, CoreError> {
        Ok(core_ops::core::verification_model::VerificationReadinessAcquisition {
            guest: guest.clone(),
            evidence: core_ops::core::verification_model::VerificationReadinessEvidence {
                source: "synthetic".to_string(),
                accepted_record: Some(core_ops::core::verification_model::VerificationReadinessRecord {
                    run_id: "run-setup-failure".to_string(),
                    token: "token".to_string(),
                    ip: "192.0.2.10".to_string(),
                    hostname: None,
                    ts: None,
                }),
                rejected_records: Vec::new(),
                final_status: "accepted".to_string(),
                failure_summary: None,
            },
        })
    }

    fn destroy_guest(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
    ) -> Result<(), CoreError> {
        self.destroy_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Default)]
struct SetupFailureGuestBoundary;

impl VerificationGuestBoundary for SetupFailureGuestBoundary {
    fn wait_ready(
        &self,
        guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        timeout: &str,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Ok(core_ops::core::verification_model::GuestCommandOutput {
            status_code: 0,
            stdout: format!("{} ready within {timeout}", guest.guest_name),
            stderr: String::new(),
        })
    }

    fn run_command(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        _command: &str,
        _timeout: Option<&str>,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Ok(core_ops::core::verification_model::GuestCommandOutput {
            status_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
        })
    }

    fn copy_to_guest(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        _local_path: &std::path::Path,
        _remote_path: &str,
        _recursive: bool,
        _executable: bool,
    ) -> Result<(), CoreError> {
        Err(CoreError::new(
            FailureClass::Apply,
            "simulated guest copy failure",
        ))
    }
}

#[test]
fn env_backed_setup_error_tears_down_guest_when_retention_is_disabled() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let destroy_calls = Arc::new(AtomicUsize::new(0));
    let libvirt = SetupFailureLibvirtBoundary {
        destroy_calls: Arc::clone(&destroy_calls),
    };
    let guest = SetupFailureGuestBoundary;
    let collector = ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let temp_binary = tempfile::NamedTempFile::new().expect("temp binary");
    std::env::set_var("CORE_OPS_VERIFY_CORE_OPS_BIN", temp_binary.path());

    let result = execute_scenario(
        &scenario,
        VerificationRunMode::Local,
        "run-setup-failure",
        &context,
        false,
        false,
    );

    std::env::remove_var("CORE_OPS_VERIFY_CORE_OPS_BIN");

    let err = result.expect_err("setup failure");
    assert!(err.message.contains("simulated guest copy failure"));
    assert_eq!(destroy_calls.load(Ordering::SeqCst), 1);
}

#[derive(Default)]
struct InfrastructureFailureGuestBoundary;

impl VerificationGuestBoundary for InfrastructureFailureGuestBoundary {
    fn wait_ready(
        &self,
        guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        timeout: &str,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Ok(core_ops::core::verification_model::GuestCommandOutput {
            status_code: 0,
            stdout: format!("{} ready within {timeout}", guest.guest_name),
            stderr: String::new(),
        })
    }

    fn run_command(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        command: &str,
        _timeout: Option<&str>,
    ) -> Result<core_ops::core::verification_model::GuestCommandOutput, CoreError> {
        Ok(core_ops::core::verification_model::GuestCommandOutput {
            status_code: 1,
            stdout: format!("{command}: missing"),
            stderr: String::new(),
        })
    }

    fn copy_to_guest(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
        _local_path: &std::path::Path,
        _remote_path: &str,
        _recursive: bool,
        _executable: bool,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

#[test]
fn expected_infrastructure_failure_counts_as_passed_scenario() {
    let mut scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    scenario.scenario_id = "expected-infrastructure-failure".to_string();
    scenario.title = "Expected infrastructure failure passes contract".to_string();
    scenario.description =
        "A guest command failure counts as passed when infrastructure failure is expected."
            .to_string();
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
            step_id: "missing-guest-command".to_string(),
            step_type: VerificationStepType::GuestCommand,
            target: VerificationStepTarget::Guest,
            action: None,
            command: Some("sudo /core-ops-verification/definitely-missing-command".to_string()),
            legacy_command_or_action: None,
            expected_exit_behavior: None,
            timeout_override: None,
        },
    ];
    scenario.assertions = vec![build_assertion(
        "boot-succeeded",
        "step_exit_code_is",
        "boot",
        "0",
        "Infrastructure-failure scenario did not boot successfully.",
    )];
    scenario.expected_outcome = Some(VerificationRunOutcome::InfrastructureFailure);

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = ReadinessOutcomeLibvirtBoundary {
        evidence: VerificationReadinessEvidence {
            source: "serial-console".to_string(),
            accepted_record: Some(
                core_ops::core::verification_model::VerificationReadinessRecord {
                    run_id: "run-current".to_string(),
                    token: "token-current".to_string(),
                    ip: "192.0.2.40".to_string(),
                    hostname: None,
                    ts: None,
                },
            ),
            rejected_records: Vec::new(),
            final_status: "accepted".to_string(),
            failure_summary: None,
        },
    };
    let guest = InfrastructureFailureGuestBoundary;
    let collector = ArtifactCollector;
    let context = VerificationExecutionContext {
        workspace: workspace.path(),
        artifacts_root: artifacts.path(),
        libvirt: &libvirt,
        guest_boundary: &guest,
        artifact_boundary: &collector,
    };
    let temp_binary = tempfile::NamedTempFile::new().expect("temp binary");
    std::env::set_var("CORE_OPS_VERIFY_CORE_OPS_BIN", temp_binary.path());

    let view = execute_scenario(
        &scenario,
        VerificationRunMode::Ci,
        "run-expected-infra-failure",
        &context,
        false,
        false,
    )
    .expect("execute");
    std::env::remove_var("CORE_OPS_VERIFY_CORE_OPS_BIN");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Passed);
    assert_eq!(view.failure_summary, None);
    assert!(view
        .warnings
        .iter()
        .any(|warning| warning.contains("expected scenario outcome `infrastructure_failure` observed as designed")));
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

fn build_readiness_guest(
    workspace_root: &std::path::Path,
    console_log_path: &std::path::Path,
) -> core_ops::core::verification_model::LibvirtGuestHandle {
    core_ops::core::verification_model::LibvirtGuestHandle {
        guest_name: "readiness-guest".to_string(),
        domain_name: "readiness-domain".to_string(),
        ssh_target: "core@0.0.0.0".to_string(),
        connection_uri: "qemu:///system".to_string(),
        workspace_root: workspace_root.display().to_string(),
        env_backed: true,
        network_mode: Some("dhcp".to_string()),
        vm_host: None,
        ssh_user: Some("core".to_string()),
        ignition_path: None,
        local_butane_path: None,
        local_ignition_path: None,
        volume_name: Some("readiness.qcow2".to_string()),
        assigned_ip: None,
        lease_path: None,
        rendered_network_config: None,
        serial_log_path: Some(console_log_path.display().to_string()),
        qemu_launch_log_path: None,
        readiness_payload: Some(VerificationGuestReadinessPayload {
            run_id: "run-current".to_string(),
            token: "token-current".to_string(),
            console_marker: VERIFICATION_READINESS_MARKER.to_string(),
            service_name: VERIFICATION_READINESS_SERVICE_NAME.to_string(),
            script_path: VERIFICATION_READINESS_SCRIPT_PATH.to_string(),
        }),
        readiness_evidence: None,
    }
}

#[test]
fn serial_console_readiness_ignores_stale_and_malformed_records_before_valid_acceptance() {
    let workspace = tempfile::tempdir().expect("workspace");
    let console = workspace.path().join("console.log");
    fs::write(
        &console,
        concat!(
            "noise\n",
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-old\",\"token\":\"token-old\",\"ip\":\"192.0.2.20\"}\n",
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\"}\n",
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\",\"ip\":\"192.0.2.30\"}\n"
        ),
    )
    .expect("write console");
    let libvirt = LibvirtCommandRunner {
        env_backed: true,
        ..LibvirtCommandRunner::default()
    };
    let guest = build_readiness_guest(workspace.path(), &console);
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");

    let acquisition = libvirt
        .acquire_guest_readiness(&scenario, &guest)
        .expect("readiness acquisition");

    assert_eq!(acquisition.evidence.source, "serial-console");
    assert_eq!(acquisition.evidence.final_status, "accepted");
    assert_eq!(
        acquisition
            .evidence
            .accepted_record
            .as_ref()
            .expect("accepted")
            .ip,
        "192.0.2.30"
    );
    assert_eq!(acquisition.evidence.rejected_records.len(), 2);
}

#[test]
fn serial_console_readiness_rejects_previous_run_history_replay() {
    let workspace = tempfile::tempdir().expect("workspace");
    let console = workspace.path().join("console.log");
    fs::write(
        &console,
        concat!(
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-previous\",\"token\":\"token-previous\",\"ip\":\"192.0.2.11\"}\n",
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\",\"ip\":\"192.0.2.31\"}\n"
        ),
    )
    .expect("write console");
    let libvirt = LibvirtCommandRunner {
        env_backed: true,
        ..LibvirtCommandRunner::default()
    };
    let guest = build_readiness_guest(workspace.path(), &console);
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");

    let acquisition = libvirt
        .acquire_guest_readiness(&scenario, &guest)
        .expect("readiness acquisition");

    assert_eq!(
        acquisition
            .evidence
            .accepted_record
            .as_ref()
            .expect("accepted")
            .ip,
        "192.0.2.31"
    );
    assert_eq!(
        acquisition.evidence.rejected_records[0].kind,
        VerificationReadinessRejectionKind::Stale
    );
}

#[test]
fn serial_console_readiness_retries_transient_console_read_failures() {
    let workspace = tempfile::tempdir().expect("workspace");
    let console = workspace.path().join("console.log");
    let console_for_writer = console.clone();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        fs::write(
            &console_for_writer,
            "CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\",\"ip\":\"192.0.2.33\"}\n",
        )
        .expect("write console");
    });

    let libvirt = LibvirtCommandRunner {
        env_backed: true,
        ..LibvirtCommandRunner::default()
    };
    let guest = build_readiness_guest(workspace.path(), &console);
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
    timeouts.readiness_timeout = "4s".to_string();
    overrides.timeouts = Some(timeouts);
    scenario.policy_overrides = Some(overrides);

    let acquisition = libvirt
        .acquire_guest_readiness(&scenario, &guest)
        .expect("readiness acquisition");
    writer.join().expect("writer");

    assert_eq!(acquisition.evidence.final_status, "accepted");
    assert_eq!(
        acquisition
            .evidence
            .accepted_record
            .as_ref()
            .expect("accepted")
            .ip,
        "192.0.2.33"
    );
}

#[test]
fn serial_console_readiness_tolerates_non_utf8_console_bytes() {
    let workspace = tempfile::tempdir().expect("workspace");
    let console = workspace.path().join("console.log");
    let mut bytes = Vec::from(&b"\x1b[0;32mgarbage-prefix\xff\xfe\r\n"[..]);
    bytes.extend_from_slice(
        b"CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\",\"ip\":\"192.0.2.34\"}\n",
    );
    std::fs::write(&console, bytes).expect("write console");

    let libvirt = LibvirtCommandRunner {
        env_backed: true,
        ..LibvirtCommandRunner::default()
    };
    let guest = build_readiness_guest(workspace.path(), &console);
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");

    let acquisition = libvirt
        .acquire_guest_readiness(&scenario, &guest)
        .expect("readiness acquisition");

    assert_eq!(acquisition.evidence.final_status, "accepted");
    assert_eq!(
        acquisition
            .evidence
            .accepted_record
            .as_ref()
            .expect("accepted")
            .ip,
        "192.0.2.34"
    );
}

#[test]
fn serial_console_readiness_precedes_fallback_when_valid_record_exists() {
    let workspace = tempfile::tempdir().expect("workspace");
    let console = workspace.path().join("console.log");
    fs::write(
        &console,
        "CORE_OPS_VERIFY_READY {\"run_id\":\"run-current\",\"token\":\"token-current\",\"ip\":\"192.0.2.32\"}\n",
    )
    .expect("write console");
    std::env::set_var("CORE_OPS_VERIFY_ALLOW_ARP_FALLBACK", "true");
    let libvirt = LibvirtCommandRunner {
        env_backed: true,
        ..LibvirtCommandRunner::default()
    };
    let guest = build_readiness_guest(workspace.path(), &console);
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");

    let acquisition = libvirt
        .acquire_guest_readiness(&scenario, &guest)
        .expect("readiness acquisition");
    std::env::remove_var("CORE_OPS_VERIFY_ALLOW_ARP_FALLBACK");

    assert_eq!(acquisition.evidence.source, "serial-console");
    assert_eq!(acquisition.evidence.final_status, "accepted");
}

struct ReadinessOutcomeLibvirtBoundary {
    evidence: VerificationReadinessEvidence,
}

impl VerificationLibvirtBoundary for ReadinessOutcomeLibvirtBoundary {
    fn create_guest(
        &self,
        _scenario: &core_ops::core::verification_model::VerificationScenarioDefinition,
        workspace_root: &std::path::Path,
    ) -> Result<core_ops::core::verification_model::LibvirtGuestHandle, CoreError> {
        Ok(build_readiness_guest(
            workspace_root,
            &workspace_root.join("console.log"),
        ))
    }

    fn acquire_guest_readiness(
        &self,
        _scenario: &core_ops::core::verification_model::VerificationScenarioDefinition,
        guest: &core_ops::core::verification_model::LibvirtGuestHandle,
    ) -> Result<VerificationReadinessAcquisition, CoreError> {
        let mut guest = guest.clone();
        if let Some(record) = &self.evidence.accepted_record {
            guest.assigned_ip = Some(record.ip.clone());
            guest.ssh_target = format!("core@{}", record.ip);
        }
        guest.readiness_evidence = Some(self.evidence.clone());
        Ok(VerificationReadinessAcquisition {
            guest,
            evidence: self.evidence.clone(),
        })
    }

    fn destroy_guest(
        &self,
        _guest: &core_ops::core::verification_model::LibvirtGuestHandle,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

#[test]
fn execute_scenario_reports_missing_readiness_timeout_as_timeout_view() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = ReadinessOutcomeLibvirtBoundary {
        evidence: VerificationReadinessEvidence {
            source: "serial-console".to_string(),
            accepted_record: None,
            rejected_records: Vec::new(),
            final_status: "timed_out".to_string(),
            failure_summary: Some(
                "no valid serial-console readiness record was accepted before the readiness deadline"
                    .to_string(),
            ),
        },
    };
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
        "run-readiness-timeout",
        &context,
        false,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::Timeout);
    assert_eq!(view.failure_summary.as_deref(), libvirt.evidence.failure_summary.as_deref());
    assert_eq!(
        view.readiness_evidence
            .as_ref()
            .expect("readiness")
            .final_status,
        "timed_out"
    );
}

#[test]
fn execute_scenario_reports_readiness_rejections_as_infrastructure_failure() {
    let scenario = load_scenario_definition(&fixture_path(
        "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
    ))
    .expect("scenario");
    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let libvirt = ReadinessOutcomeLibvirtBoundary {
        evidence: VerificationReadinessEvidence {
            source: "serial-console".to_string(),
            accepted_record: None,
            rejected_records: vec![VerificationReadinessRejection {
                kind: VerificationReadinessRejectionKind::Malformed,
                summary: "readiness record does not contain a usable IPv4 address".to_string(),
                raw_line: Some("CORE_OPS_VERIFY_READY {}".to_string()),
            }],
            final_status: "invalid".to_string(),
            failure_summary: Some(
                "serial-console readiness was rejected before guest access could begin"
                    .to_string(),
            ),
        },
    };
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
        "run-readiness-infra",
        &context,
        false,
        false,
    )
    .expect("execute");

    assert_eq!(view.overall_outcome, VerificationRunOutcome::InfrastructureFailure);
    assert_eq!(
        view.readiness_evidence
            .as_ref()
            .expect("readiness")
            .rejected_records
            .len(),
        1
    );
}

#[test]
fn verification_run_humane_and_json_preserve_same_outcome_semantics() {
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
        VerificationRunMode::Ci,
        "run-parity-failure",
        &context,
        false,
        false,
    )
    .expect("execute");

    let human = format_verification_run_report(&view);
    let json: serde_json::Value =
        serde_json::from_str(&format_verification_run_json(&view)).expect("valid json");

    assert!(human.contains("Verification run verify-idempotent-frontend"));
    assert!(human.contains("Outcome: assertion_failure"));
    assert!(human.contains("Failure: one or more verification assertions failed"));
    assert!(human.contains(&view.artifact_bundle.bundle_path));

    assert_eq!(json["view_kind"], "verification_run");
    assert_eq!(json["overall_outcome"], "assertion_failure");
    assert_eq!(
        json["scenario_outcomes"][0]["scenario_id"],
        "verify-idempotent-frontend"
    );
    assert_eq!(
        json["scenario_outcomes"][0]["failure_summary"],
        "one or more verification assertions failed"
    );
    assert_eq!(
        json["artifacts"]["bundle_path"],
        serde_json::Value::String(view.artifact_bundle.bundle_path.clone())
    );
    assert_eq!(
        json["scenario_outcomes"][0]["readiness_evidence"]["source"],
        "synthetic"
    );
}
