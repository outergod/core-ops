use crate::core::types::{AuditRecord, DiffItem, PlanAction, ReconciliationPlan};

pub fn build_audit_record(run_id: &str, diffs: Vec<DiffItem>, plan: &ReconciliationPlan) -> AuditRecord {
    AuditRecord {
        record_id: format!("audit:{}", run_id),
        run_id: run_id.to_string(),
        diffs,
        plan_summary: summarize_plan(plan),
        actions_applied: plan.actions.clone(),
        verification_results: Vec::new(),
        operator_messages: Vec::new(),
    }
}

fn summarize_plan(plan: &ReconciliationPlan) -> String {
    format!(
        "plan {} with {} actions",
        plan.plan_id,
        plan.actions.len()
    )
}

pub fn summarize_actions(actions: &[PlanAction]) -> String {
    format!("{} actions", actions.len())
}
