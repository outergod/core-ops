use crate::core::types::{DiffItem, QuadletType, ReconciliationPlan};

pub fn format_plan_report(plan: &ReconciliationPlan, diffs: &[DiffItem]) -> String {
    let mut output = String::new();
    output.push_str(&format!("plan {} with {} actions\n", plan.plan_id, plan.actions.len()));
    output.push_str(&format!("diffs {}\n", diffs.len()));
    for diff in diffs {
        let quadlet_type = diff
            .desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|w| w.quadlet_type.clone());
        let quadlet_label = quadlet_type_label(quadlet_type);
        output.push_str(&format!(
            "- {:?}: {} [{}]\n",
            diff.kind, diff.name, quadlet_label
        ));
    }
    output.push_str("actions\n");
    for action in &plan.actions {
        output.push_str(&format!("- {:?}: {}\n", action.action_type, action.target));
    }
    output
}

fn quadlet_type_label(quadlet_type: Option<QuadletType>) -> &'static str {
    match quadlet_type {
        Some(QuadletType::Container) => "container",
        Some(QuadletType::Socket) => "socket",
        Some(QuadletType::Volume) => "volume",
        Some(QuadletType::Pod) => "pod",
        Some(QuadletType::Network) => "network",
        None => "unknown",
    }
}
