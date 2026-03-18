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
    let audit = build_audit_record(&result.run.run_id, diffs.clone(), &result.plan);
    let event = build_audit_event(&result.run, Some(&result.plan));

    Ok(PlanOutput {
        summary: format_plan_output(&result.plan, &diffs),
        audit_record: audit,
        audit_event: event,
    })
}

pub fn format_plan_output(
    plan: &crate::core::types::ReconciliationPlan,
    diffs: &[crate::core::types::DiffItem],
) -> String {
    let mut output = String::new();
    output.push_str(&format!("plan {} with {} actions\n", plan.plan_id, plan.actions.len()));
    output.push_str(&format!("diffs {}\n", diffs.len()));
    for diff in diffs {
        output.push_str(&format!("- {:?}: {}\n", diff.kind, diff.name));
    }
    output.push_str("actions\n");
    for action in &plan.actions {
        output.push_str(&format!("- {:?}: {}\n", action.action_type, action.target));
    }
    output
}
