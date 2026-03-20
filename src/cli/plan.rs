use crate::cli::report::format_plan_report;
use crate::core::audit::{build_audit_event, build_audit_record, AuditEvent};
use crate::core::errors::CoreError;
use crate::core::reconcile::{reconcile_plan, ReconcileDependencies};
use crate::core::types::AuditRecord;

pub struct PlanOutput {
    pub summary: String,
    pub audit_record: AuditRecord,
    pub audit_event: AuditEvent,
}

pub fn plan(deps: &ReconcileDependencies<'_>) -> Result<PlanOutput, CoreError> {
    let result = reconcile_plan(deps)?;
    let diffs = result.diffs;
    let audit = build_audit_record(&result.run.run_id, diffs.clone(), &result.plan, Vec::new());
    let event = build_audit_event(&result.run, Some(&result.plan));

    Ok(PlanOutput {
        summary: format_plan_report(&result.plan, &diffs),
        audit_record: audit,
        audit_event: event,
    })
}
