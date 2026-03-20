use core_ops::core::audit::{build_audit_event, format_audit_event_json};
use core_ops::core::types::{FailureClass, ReconcileMode, ReconcileRun, RunStatus};

#[test]
fn journald_audit_event_contains_summary_and_ids() {
    let run = ReconcileRun {
        run_id: "run:test".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Success,
        failure_class: Some(FailureClass::Apply),
        summary: "converged".to_string(),
    };
    let event = build_audit_event(&run, None);
    let payload = format_audit_event_json(&event);

    assert!(payload.contains("\"run_id\":\"run:test\""));
    assert!(payload.contains("\"summary\":\"converged\""));
}
