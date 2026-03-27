use crate::cli::report::{
    format_deterministic_plan_json, format_deterministic_plan_report,
    format_deterministic_plan_report_with_options,
};
use crate::core::audit::{build_audit_event, build_audit_record, summarize_evaluation, AuditEvent};
use crate::core::evaluate::build_desired_snapshot_from_state;
use crate::core::errors::CoreError;
use crate::core::reconcile::{reconcile_deterministic_plan, reconcile_plan, ReconcileDependencies};
use crate::core::types::AuditRecord;
use crate::io::observed::build_observed_snapshot;
use crate::io::repo::HOST_OVERRIDE_ENV;
use crate::io::state::{read_persisted_state, resolve_state_file};

pub struct PlanOutput {
    pub summary: String,
    pub machine: String,
    pub audit_record: AuditRecord,
    pub audit_event: AuditEvent,
}

pub struct DeterministicPlanOutput {
    pub summary: String,
    pub machine: String,
}

pub fn plan(deps: &ReconcileDependencies<'_>, verbose: bool) -> Result<PlanOutput, CoreError> {
    let result = reconcile_plan(deps)?;
    let observed = (deps.read_observed)(&result.desired)?;
    let scope_id = scope_id_for_observed(&observed);
    let desired_snapshot = build_desired_snapshot_from_state(&result.desired, &scope_id);
    let observed_snapshot = build_observed_snapshot(&observed, Some(&result.desired), &scope_id);
    let mut deterministic =
        reconcile_deterministic_plan(&desired_snapshot, None, &observed_snapshot)?;
    if deterministic.plan.baseline_revision_id.is_none() {
        if let Some(revision) = last_applied_revision_from_state() {
            deterministic.plan.baseline_revision_id = Some(revision);
        }
    }
    let diffs = result.diffs;
    let mut audit = build_audit_record(&result.run.run_id, diffs.clone(), &result.plan, Vec::new());
    audit
        .operator_messages
        .push(summarize_evaluation(&result.desired));
    if !result.desired.mount_declarations.is_empty() {
        audit.operator_messages.push(format!(
            "mounts: native-artifacts={}, dependencies={}",
            result.desired.mount_declarations.len(),
            result.desired.mount_dependencies.len()
        ));
    }
    let event = build_audit_event(&result.run, Some(&result.plan), &[], None);

    Ok(PlanOutput {
        summary: format_deterministic_plan_report_with_options(&deterministic.plan, verbose),
        machine: format_deterministic_plan_json(&deterministic.plan),
        audit_record: audit,
        audit_event: event,
    })
}

pub fn render_deterministic_plan(
    plan: &crate::core::types::DeterministicReconciliationPlan,
) -> DeterministicPlanOutput {
    // The machine-readable plan view is authoritative; the summary is a projection of it.
    DeterministicPlanOutput {
        summary: format_deterministic_plan_report(plan),
        machine: format_deterministic_plan_json(plan),
    }
}

fn scope_id_for_observed(observed: &crate::core::types::ObservedState) -> String {
    observed
        .host_info
        .as_ref()
        .map(|host| format!("host:{}", host.hostname))
        .or_else(|| std::env::var(HOST_OVERRIDE_ENV).ok().filter(|host| !host.is_empty()).map(|host| format!("host:{host}")))
        .or_else(default_host_scope_id)
        .unwrap_or_else(|| "scope:default".to_string())
}

fn default_host_scope_id() -> Option<String> {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if result != 0 {
        return None;
    }
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let hostname = String::from_utf8_lossy(&buf[..len]).trim().to_string();
    (!hostname.is_empty()).then(|| format!("host:{hostname}"))
}

fn last_applied_revision_from_state() -> Option<String> {
    let state_path = resolve_state_file(None);
    read_persisted_state(&state_path)
        .ok()
        .flatten()
        .and_then(|state| state.reconciliation.last_applied_revision)
}
