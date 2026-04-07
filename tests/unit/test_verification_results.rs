use core_ops::core::types::{
    VerificationAssertionStatus, VerificationRunOutcome, VerificationStepStatus,
};
use core_ops::core::verification_eval::{classify_run_outcome, classify_scenario_outcome};
use core_ops::core::verification_model::{
    VerificationAssertionResult, VerificationScenarioOutcome, VerificationStepResult,
    VerificationStepType,
};

#[test]
fn scenario_outcome_prefers_step_failures_over_assertions() {
    let outcome = classify_scenario_outcome(
        &[VerificationStepResult {
            step_id: "boot".to_string(),
            step_type: VerificationStepType::Boot,
            status: VerificationStepStatus::Failed,
            details: Some("guest never booted".to_string()),
            command: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            duration_ms: None,
        }],
        &[VerificationAssertionResult {
            assertion_id: "never-reached".to_string(),
            status: VerificationAssertionStatus::NotEvaluated,
            observed_value: None,
            evidence_refs: Vec::new(),
        }],
    );

    assert_eq!(outcome, VerificationRunOutcome::InfrastructureFailure);
}

#[test]
fn scenario_outcome_maps_failed_coreops_action_to_assertion_failure() {
    let outcome = classify_scenario_outcome(
        &[VerificationStepResult {
            step_id: "apply".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            status: VerificationStepStatus::Failed,
            details: Some("failed during Applying".to_string()),
            command: Some("sudo core-ops apply".to_string()),
            exit_code: Some(1),
            stdout: Some("failed during Applying".to_string()),
            stderr: None,
            duration_ms: None,
        }],
        &[VerificationAssertionResult {
            assertion_id: "never-reached".to_string(),
            status: VerificationAssertionStatus::NotEvaluated,
            observed_value: None,
            evidence_refs: Vec::new(),
        }],
    );

    assert_eq!(outcome, VerificationRunOutcome::AssertionFailure);
}

#[test]
fn scenario_outcome_maps_failed_assertions_to_assertion_failure() {
    let outcome = classify_scenario_outcome(
        &[VerificationStepResult {
            step_id: "apply".to_string(),
            step_type: VerificationStepType::CoreopsAction,
            status: VerificationStepStatus::Passed,
            details: None,
            command: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            duration_ms: None,
        }],
        &[VerificationAssertionResult {
            assertion_id: "no-pending-change".to_string(),
            status: VerificationAssertionStatus::Failed,
            observed_value: Some("1 update".to_string()),
            evidence_refs: vec!["artifacts/assertions/no-pending-change.json".to_string()],
        }],
    );

    assert_eq!(outcome, VerificationRunOutcome::AssertionFailure);
}

#[test]
fn overall_run_outcome_prefers_non_passed_failure_classes() {
    let overall = classify_run_outcome(&[
        VerificationScenarioOutcome {
            scenario_id: "a".to_string(),
            revision_under_test: "demo-uat-v1".to_string(),
            outcome: VerificationRunOutcome::Passed,
            step_results: Vec::new(),
            assertion_results: Vec::new(),
            failure_summary: None,
        },
        VerificationScenarioOutcome {
            scenario_id: "b".to_string(),
            revision_under_test: "demo-uat-v2".to_string(),
            outcome: VerificationRunOutcome::Timeout,
            step_results: Vec::new(),
            assertion_results: Vec::new(),
            failure_summary: Some("readiness timeout".to_string()),
        },
    ]);

    assert_eq!(overall, VerificationRunOutcome::Timeout);
}

#[test]
fn overall_run_outcome_precedence_is_order_independent() {
    let infrastructure = VerificationScenarioOutcome {
        scenario_id: "infra".to_string(),
        revision_under_test: "demo-uat-v1".to_string(),
        outcome: VerificationRunOutcome::InfrastructureFailure,
        step_results: Vec::new(),
        assertion_results: Vec::new(),
        failure_summary: Some("guest boot failed".to_string()),
    };
    let timeout = VerificationScenarioOutcome {
        scenario_id: "timeout".to_string(),
        revision_under_test: "demo-uat-v2".to_string(),
        outcome: VerificationRunOutcome::Timeout,
        step_results: Vec::new(),
        assertion_results: Vec::new(),
        failure_summary: Some("readiness timeout".to_string()),
    };
    let assertion = VerificationScenarioOutcome {
        scenario_id: "assertion".to_string(),
        revision_under_test: "demo-uat-v3".to_string(),
        outcome: VerificationRunOutcome::AssertionFailure,
        step_results: Vec::new(),
        assertion_results: Vec::new(),
        failure_summary: Some("assertion failed".to_string()),
    };

    let first = classify_run_outcome(&[
        infrastructure.clone(),
        timeout.clone(),
        assertion.clone(),
    ]);
    let second = classify_run_outcome(&[assertion, infrastructure, timeout]);

    assert_eq!(first, VerificationRunOutcome::InfrastructureFailure);
    assert_eq!(second, VerificationRunOutcome::InfrastructureFailure);
}
