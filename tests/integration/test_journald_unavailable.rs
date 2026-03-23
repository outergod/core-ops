use core_ops::core::audit::build_audit_event;
use core_ops::core::types::{ReconcileMode, ReconcileRun, RunStatus};
use core_ops::io::audit::emit_journal_event;

#[test]
fn journald_unavailable_does_not_fail_emit() {
    let run = ReconcileRun {
        run_id: "run:journald".to_string(),
        mode: ReconcileMode::Apply,
        status: RunStatus::Success,
        failure_class: None,
        summary: "converged".to_string(),
    };
    let event = build_audit_event(&run, None, &[]);
    emit_journal_event(&event).expect("emit audit event");
}
