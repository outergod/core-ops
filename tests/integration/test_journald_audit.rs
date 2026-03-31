use core_ops::core::audit::{build_audit_event, format_audit_event_json};
use core_ops::core::types::{
    FailureClass, PlanAction, PlanActionType, ReconcileMode, ReconcileRun, ReconciliationPlan,
    RunStatus, VerificationResult, VerificationStatus,
};
use core_ops::io::audit::journal_target;

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

#[test]
fn journald_audit_mount_payloads_cover_success_degraded_and_busy_removal() {
    let success_run = ReconcileRun {
        run_id: "run:mount-success".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Success,
        failure_class: None,
        summary: "mount converged".to_string(),
    };
    let degraded_run = ReconcileRun {
        run_id: "run:mount-degraded".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Failure,
        failure_class: Some(FailureClass::Verify),
        summary: "mount degraded".to_string(),
    };
    let busy_run = ReconcileRun {
        run_id: "run:mount-busy".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Failure,
        failure_class: Some(FailureClass::Apply),
        summary: "busy mount removal".to_string(),
    };
    let plan = ReconciliationPlan {
        plan_id: "plan:mount".to_string(),
        desired_revision_id: "rev".to_string(),
        observed_revision_id: None,
        actions: vec![PlanAction {
            action_type: PlanActionType::RemoveQuadlet,
            target: "srv-immich-media.mount".to_string(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        }],
        safety_checks: Vec::new(),
        expected_outcomes: Vec::new(),
    };
    let degraded_results = vec![VerificationResult {
        target: "srv-immich-media.mount".to_string(),
        status: VerificationStatus::Failure,
        details: Some("degraded: mount target not mounted".to_string()),
    }];
    let success = build_audit_event(&success_run, Some(&plan), &[], None);
    let degraded = build_audit_event(&degraded_run, Some(&plan), &degraded_results, None);
    let busy = build_audit_event(&busy_run, Some(&plan), &[], None);

    let success_payload = format_audit_event_json(&success);
    let degraded_payload = format_audit_event_json(&degraded);
    let busy_payload = format_audit_event_json(&busy);

    assert_eq!(journal_target(&success), "audit.mount");
    assert_eq!(journal_target(&degraded), "audit.mount");
    assert_eq!(journal_target(&busy), "audit.mount");
    assert!(success_payload.contains("\"summary\":\"mount converged\""));
    assert!(degraded_payload.contains("\"failed_artifacts\":[\"srv-immich-media.mount\"]"));
    assert!(degraded_payload.contains("\"failure_class\":\"verify\""));
    assert!(busy_payload.contains("\"failure_reason\":\"busy mount removal\""));
    assert!(busy_payload.contains("\"failure_class\":\"apply\""));
}
