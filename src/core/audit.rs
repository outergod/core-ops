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

pub fn format_audit_record(record: &AuditRecord) -> String {
    let mut output = String::new();
    output.push_str(&format!("record {}\n", record.record_id));
    output.push_str(&format!("run {}\n", record.run_id));
    output.push_str(&format!("{}\n", record.plan_summary));
    output.push_str(&format!(
        "diffs {}\n",
        record.diffs.len()
    ));
    output.push_str(&format!(
        "actions {}\n",
        record.actions_applied.len()
    ));
    output
}
