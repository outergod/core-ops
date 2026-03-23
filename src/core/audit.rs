use crate::core::types::{
    AuditRecord, DesiredState, DiffItem, FailureClass, PersistedProvenanceState, PlanAction,
    QuadletType, ReconcileRun, ReconciliationPlan, ReconciliationStatus, RunStatus,
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

pub fn summarize_evaluation(desired: &DesiredState) -> String {
    let mut socket_dropins = 0;
    let mut sockets = 0;
    let mut containers = 0;
    let mut volumes = 0;
    for workload in &desired.workloads {
        match workload.quadlet_type {
            QuadletType::SocketDropIn => socket_dropins += 1,
            QuadletType::Socket => sockets += 1,
            QuadletType::Container => containers += 1,
            QuadletType::Volume => volumes += 1,
            _ => {}
        }
    }
    format!(
        "evaluation: workloads={}, containers={}, volumes={}, sockets={}, socket_dropins={}",
        desired.workloads.len(),
        containers,
        volumes,
        sockets,
        socket_dropins
    )
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
    pub controller_version: Option<String>,
    pub controller_revision: Option<String>,
    pub desired_repository: Option<String>,
    pub desired_requested_ref: Option<String>,
    pub desired_observed_revision: Option<String>,
    pub reconciliation_generation: Option<u64>,
    pub reconciliation_status: Option<String>,
    pub attempted_revision: Option<String>,
    pub applied_revision: Option<String>,
}

pub fn build_audit_event(
    run: &ReconcileRun,
    plan: Option<&ReconciliationPlan>,
    verification_results: &[VerificationResult],
    provenance: Option<&PersistedProvenanceState>,
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
    let (
        controller_version,
        controller_revision,
        desired_repository,
        desired_requested_ref,
        desired_observed_revision,
        reconciliation_generation,
        reconciliation_status,
        attempted_revision,
        applied_revision,
    ) = match provenance {
        Some(state) => (
            state.controller.version.clone(),
            state.controller.revision.clone(),
            Some(state.desired_state.repository.clone()),
            Some(state.desired_state.requested_ref.clone()),
            state.desired_state.last_observed_revision.clone(),
            Some(state.reconciliation.generation),
            Some(reconciliation_status_label(&state.reconciliation.status).to_string()),
            state.reconciliation.last_attempted_revision.clone(),
            state.reconciliation.last_applied_revision.clone(),
        ),
        None => (None, None, None, None, None, None, None, None, None),
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
        controller_version,
        controller_revision,
        desired_repository,
        desired_requested_ref,
        desired_observed_revision,
        reconciliation_generation,
        reconciliation_status,
        attempted_revision,
        applied_revision,
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
    let controller_version = format_optional_string(event.controller_version.as_deref());
    let controller_revision = format_optional_string(event.controller_revision.as_deref());
    let desired_repository = format_optional_string(event.desired_repository.as_deref());
    let desired_requested_ref = format_optional_string(event.desired_requested_ref.as_deref());
    let desired_observed_revision =
        format_optional_string(event.desired_observed_revision.as_deref());
    let reconciliation_generation = event
        .reconciliation_generation
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    let reconciliation_status =
        format_optional_string(event.reconciliation_status.as_deref());
    let attempted_revision = format_optional_string(event.attempted_revision.as_deref());
    let applied_revision = format_optional_string(event.applied_revision.as_deref());
    format!(
        "{{\"run_id\":\"{}\",\"plan_id\":{},\"plan_summary\":{},\"action_count\":{},\"status\":\"{}\",\"failure_class\":{},\"failed_artifacts\":{},\"failure_reason\":{},\"summary\":\"{}\",\"controller_version\":{},\"controller_revision\":{},\"desired_repository\":{},\"desired_requested_ref\":{},\"desired_observed_revision\":{},\"reconciliation_generation\":{},\"reconciliation_status\":{},\"attempted_revision\":{},\"applied_revision\":{}}}",
        escape_json(&event.run_id),
        plan_id,
        plan_summary,
        event.action_count,
        status_label(&event.status),
        failure_class,
        failed_artifacts,
        failure_reason,
        escape_json(&event.summary),
        controller_version,
        controller_revision,
        desired_repository,
        desired_requested_ref,
        desired_observed_revision,
        reconciliation_generation,
        reconciliation_status,
        attempted_revision,
        applied_revision
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

fn reconciliation_status_label(status: &ReconciliationStatus) -> &'static str {
    match status {
        ReconciliationStatus::NeverRun => "never_run",
        ReconciliationStatus::InProgress => "in_progress",
        ReconciliationStatus::Success => "success",
        ReconciliationStatus::Failed => "failed",
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

fn format_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json(value)),
        None => "null".to_string(),
    }
}
