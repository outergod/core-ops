use crate::core::types::{DiffItem, ReconciliationPlan};

pub fn format_plan_report(plan: &ReconciliationPlan, diffs: &[DiffItem]) -> String {
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
