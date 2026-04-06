use core_ops::core::types::VerificationRunMode;
use core_ops::core::verification_eval::{
    assertion_matches_no_pending_changes, build_execution_plan, should_retain_environment,
};
use core_ops::core::verification_model::{
    parse_scenario_definition, VerificationAssertionSpec, VerificationCoreOpsAction,
    VerificationCoreOpsActionKind, VerificationScenarioStep, VerificationStepTarget,
    VerificationStepType,
};

fn scenario_fixture() -> core_ops::core::verification_model::VerificationScenarioDefinition {
    parse_scenario_definition(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/verification/scenarios/minimal-accepted.yaml"),
        )
        .expect("fixture"),
    )
    .expect("scenario")
}

#[test]
fn execution_plan_preserves_step_order_and_default_timeouts() {
    let scenario = scenario_fixture();
    let plan = build_execution_plan(&scenario, "run-1", VerificationRunMode::Local, None)
        .expect("execution plan");

    assert_eq!(plan.step_sequence.len(), 3);
    assert_eq!(plan.step_sequence[0].step_id, "boot");
    assert_eq!(plan.step_sequence[1].step_id, "apply");
    assert_eq!(plan.step_sequence[2].step_id, "reapply");
    assert_eq!(plan.step_sequence[0].effective_timeout, "300s");
    assert_eq!(plan.step_sequence[1].effective_timeout, "300s");
    assert_eq!(
        plan.step_sequence[1].command_or_action.as_deref(),
        Some("sudo core-ops apply --repo fixtures/repos/frontend --rev demo-uat-v2 --quadlet-dir /etc/containers/systemd --systemd-unit-dir /etc/systemd/system")
    );
}

#[test]
fn execution_plan_uses_override_timeout_when_present() {
    let mut scenario = scenario_fixture();
    scenario.steps[1].timeout_override = Some("45s".to_string());

    let plan = build_execution_plan(&scenario, "run-2", VerificationRunMode::Local, None)
        .expect("execution plan");
    assert_eq!(plan.step_sequence[1].effective_timeout, "45s");
}

#[test]
fn teardown_rules_only_retain_environment_in_debug_mode() {
    let scenario = scenario_fixture();

    assert!(!should_retain_environment(
        VerificationRunMode::Local,
        &scenario
    ));
    assert!(should_retain_environment(
        VerificationRunMode::Debug,
        &scenario
    ));
}

#[test]
fn no_pending_changes_accepts_real_apply_report_shape() {
    let output = "\u{1b}[1m\u{1b}[37mApply for host test\u{1b}[0m\n\
────────────────────────────────\n\n\
\u{1b}[1m\u{1b}[37mExecution\u{1b}[0m\n─────────\n\
\u{1b}[1m\u{1b}[37mSummary\u{1b}[0m\n───────\n\
1 unchanged\nOutcome: converged\n";

    assert!(assertion_matches_no_pending_changes(output));
}

#[test]
fn execution_plan_renders_supported_command_surfaces_for_public_interfaces() {
    let mut scenario = scenario_fixture();
    scenario.steps = vec![
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
    ];
    scenario.assertions = vec![VerificationAssertionSpec {
        assertion_id: "surface-contract".to_string(),
        assertion_type: "step_command_contains".to_string(),
        target: "plan-json".to_string(),
        expected_state: "--json".to_string(),
        failure_message: "plan should render json mode".to_string(),
        artifact_hints: Vec::new(),
    }];

    let plan = build_execution_plan(&scenario, "run-interfaces", VerificationRunMode::Local, None)
        .expect("execution plan");

    assert_eq!(
        plan.step_sequence[0].command_or_action.as_deref(),
        Some(
            "sudo core-ops plan --repo fixtures/repos/frontend --rev demo-uat-v2 --quadlet-dir /etc/containers/systemd --systemd-unit-dir /etc/systemd/system --json"
        )
    );
    assert_eq!(
        plan.step_sequence[1].command_or_action.as_deref(),
        Some("sudo core-ops status")
    );
    assert_eq!(
        plan.step_sequence[2].command_or_action.as_deref(),
        Some(
            "sudo core-ops agent --repo fixtures/repos/frontend --rev demo-uat-v2 --quadlet-dir /etc/containers/systemd --systemd-unit-dir /etc/systemd/system"
        )
    );
}
