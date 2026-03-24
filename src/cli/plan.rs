use crate::cli::report::format_plan_report;
use crate::core::audit::{build_audit_event, build_audit_record, summarize_evaluation, AuditEvent};
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
    let mut audit = build_audit_record(&result.run.run_id, diffs.clone(), &result.plan, Vec::new());
    audit
        .operator_messages
        .push(summarize_evaluation(&result.desired));
    if !result.desired.mount_declarations.is_empty() {
        audit.operator_messages.push(format!(
            "mounts: declarations={}, dependencies={}",
            result.desired.mount_declarations.len(),
            result.desired.mount_dependencies.len()
        ));
    }
    let event = build_audit_event(&result.run, Some(&result.plan), &[], None);

    Ok(PlanOutput {
        summary: append_mount_plan_summary(
            &format_plan_report(&result.plan, &diffs),
            &result.desired,
        ),
        audit_record: audit,
        audit_event: event,
    })
}

fn append_mount_plan_summary(base: &str, desired: &crate::core::types::DesiredState) -> String {
    if desired.mount_declarations.is_empty() {
        return base.to_string();
    }
    let mount_ids = desired
        .mount_declarations
        .iter()
        .map(|mount| mount.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let automount_ids = desired
        .mount_declarations
        .iter()
        .filter(|mount| mount.automount)
        .map(|mount| mount.id.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let mut summary = format!(
        "{base}\nmount ids: {}\nmount dependencies: {}",
        mount_ids,
        desired.mount_dependencies.len()
    );
    if !automount_ids.is_empty() {
        summary.push_str(&format!("\nautomount ids: {}", automount_ids));
    }
    summary
}
