use crate::core::types::{
    AuditRecord, DiffItem, PlanAction, ReconcileRun, ReconciliationPlan, VerificationResult,
};

pub fn build_audit_record(
    run_id: &str,
    diffs: Vec<DiffItem>,
    plan: &ReconciliationPlan,
    verification_results: Vec<VerificationResult>,
) -> AuditRecord {
    AuditRecord {
        record_id: format!("audit:{}", run_id),
        run_id: run_id.to_string(),
        diffs,
        plan_summary: summarize_plan(plan),
        actions_applied: plan.actions.clone(),
        verification_results,
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
    output.push_str(&format!(
        "verification {}\n",
        record.verification_results.len()
    ));
    output
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub run_id: String,
    pub plan_id: Option<String>,
    pub action_count: usize,
    pub summary: String,
}

pub fn build_audit_event(run: &ReconcileRun, plan: Option<&ReconciliationPlan>) -> AuditEvent {
    AuditEvent {
        run_id: run.run_id.clone(),
        plan_id: plan.map(|p| p.plan_id.clone()),
        action_count: plan.map(|p| p.actions.len()).unwrap_or(0),
        summary: run.summary.clone(),
    }
}

pub fn format_audit_event_json(event: &AuditEvent) -> String {
    let plan_id = match &event.plan_id {
        Some(plan_id) => format!("\"{}\"", escape_json(plan_id)),
        None => "null".to_string(),
    };
    format!(
        "{{\"run_id\":\"{}\",\"plan_id\":{},\"action_count\":{},\"summary\":\"{}\"}}",
        escape_json(&event.run_id),
        plan_id,
        event.action_count,
        escape_json(&event.summary)
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
}
