use core_ops::core::audit::{build_audit_event, format_audit_event_json};
use core_ops::core::types::{
    FailureClass, PlanAction, PlanActionType, ReconcileMode, ReconcileRun,
    ReconciliationPlan, RunStatus, VerificationResult, VerificationStatus,
};

#[test]
fn journald_audit_event_contains_summary_and_ids() {
    let run = ReconcileRun {
        run_id: "run:test".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Success,
        failure_class: None,
        summary: "converged".to_string(),
    };
    let plan = ReconciliationPlan {
        plan_id: "plan:test".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![PlanAction {
            action_type: PlanActionType::WriteQuadlet,
            target: "alpha.container".to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };
    let event = build_audit_event(&run, Some(&plan), &[], None);
    let payload = format_audit_event_json(&event);

    assert!(payload.contains("\"run_id\":\"run:test\""));
    assert!(payload.contains("\"status\":\"success\""));
    assert!(payload.contains("\"summary\":\"converged\""));
    assert!(payload.contains("\"plan_summary\":\"plan plan:test with 1 actions\""));
}

#[test]
fn journald_audit_event_contains_failure_details() {
    let run = ReconcileRun {
        run_id: "run:fail".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Failure,
        failure_class: Some(FailureClass::Verify),
        summary: "verification failed".to_string(),
    };
    let verification_results = vec![
        VerificationResult {
            target: "alpha.container".to_string(),
            status: VerificationStatus::Failure,
            details: Some("inactive".to_string()),
        },
        VerificationResult {
            target: "beta.socket".to_string(),
            status: VerificationStatus::Success,
            details: None,
        },
    ];

    let event = build_audit_event(&run, None, &verification_results, None);
    let payload = format_audit_event_json(&event);

    assert!(payload.contains("\"status\":\"failure\""));
    assert!(payload.contains("\"failure_class\":\"verify\""));
    assert!(payload.contains("\"failed_artifacts\":[\"alpha.container\"]"));
    assert!(payload.contains("\"failure_reason\":\"verification failed\""));
}
