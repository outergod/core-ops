use crate::core::types::{
    AuditRecord, DiffItem, FailureClass, PlanAction, ReconcileRun, ReconciliationPlan, RunStatus,
    VerificationResult, VerificationStatus,
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
    pub plan_summary: Option<String>,
    pub action_count: usize,
    pub status: RunStatus,
    pub failure_class: Option<FailureClass>,
    pub failed_artifacts: Vec<String>,
    pub failure_reason: Option<String>,
    pub summary: String,
}

pub fn build_audit_event(
    run: &ReconcileRun,
    plan: Option<&ReconciliationPlan>,
    verification_results: &[VerificationResult],
) -> AuditEvent {
    let failed_artifacts = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Failure)
        .map(|result| result.target.clone())
        .collect::<Vec<_>>();
    let failure_reason = if run.status == RunStatus::Failure {
        Some(run.summary.clone())
    } else {
        None
    };

    AuditEvent {
        run_id: run.run_id.clone(),
        plan_id: plan.map(|p| p.plan_id.clone()),
        plan_summary: plan.map(summarize_plan),
        action_count: plan.map(|p| p.actions.len()).unwrap_or(0),
        status: run.status.clone(),
        failure_class: run.failure_class.clone(),
        failed_artifacts,
        failure_reason,
        summary: run.summary.clone(),
    }
}

pub fn format_audit_event_json(event: &AuditEvent) -> String {
    let plan_id = match &event.plan_id {
        Some(plan_id) => format!("\"{}\"", escape_json(plan_id)),
        None => "null".to_string(),
    };
    let plan_summary = match &event.plan_summary {
        Some(summary) => format!("\"{}\"", escape_json(summary)),
        None => "null".to_string(),
    };
    let failure_class = match &event.failure_class {
        Some(class) => format!("\"{}\"", failure_class_label(class)),
        None => "null".to_string(),
    };
    let failure_reason = match &event.failure_reason {
        Some(reason) => format!("\"{}\"", escape_json(reason)),
        None => "null".to_string(),
    };
    let failed_artifacts = format_string_array(&event.failed_artifacts);
    format!(
        "{{\"run_id\":\"{}\",\"plan_id\":{},\"plan_summary\":{},\"action_count\":{},\"status\":\"{}\",\"failure_class\":{},\"failed_artifacts\":{},\"failure_reason\":{},\"summary\":\"{}\"}}",
        escape_json(&event.run_id),
        plan_id,
        plan_summary,
        event.action_count,
        status_label(&event.status),
        failure_class,
        failed_artifacts,
        failure_reason,
        escape_json(&event.summary)
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
}

fn status_label(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Success => "success",
        RunStatus::Failure => "failure",
    }
}

fn failure_class_label(class: &FailureClass) -> &'static str {
    match class {
        FailureClass::Validation => "validation",
        FailureClass::Plan => "plan",
        FailureClass::Apply => "apply",
        FailureClass::Verify => "verify",
        FailureClass::Transient => "transient",
    }
}

fn format_string_array(values: &[String]) -> String {
    let mut output = String::from("[");
    for (idx, value) in values.iter().enumerate() {
        if idx > 0 {
            output.push(',');
        }
        output.push('"');
        output.push_str(&escape_json(value));
        output.push('"');
    }
    output.push(']');
    output
}
