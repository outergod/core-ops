use crate::core::types::{
    DiffItem, QuadletType, ReconcileRun, ReconciliationPlan, VerificationResult,
};

pub fn format_plan_report(plan: &ReconciliationPlan, diffs: &[DiffItem]) -> String {
    let mut output = String::new();
    output.push_str(&format!("plan with {} actions\n", plan.actions.len()));
    output.push_str(&format!("diffs {}\n", diffs.len()));
    let mut type_by_name = std::collections::HashMap::new();
    for diff in diffs {
        let quadlet_type = diff
            .desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|w| w.quadlet_type.clone());
        type_by_name.insert(diff.name.clone(), quadlet_type.clone());
        let quadlet_label = quadlet_type_label(quadlet_type);
        output.push_str(&format!(
            "- {:?}: {} [{}]\n",
            diff.kind, diff.name, quadlet_label
        ));
    }
    output.push_str("actions\n");
    for action in &plan.actions {
        let quadlet_type = type_by_name.get(&action.target).cloned().flatten();
        let action_label = action_label(&action.action_type, quadlet_type.as_ref());
        output.push_str(&format!("- {}: {}\n", action_label, action.target));
    }
    output
}

pub fn append_provenance_report(base: &str, contents: Option<&str>) -> String {
    match contents {
        Some(contents) => format!("{base}\n{}", crate::cli::status::format_status_text(contents)),
        None => base.to_string(),
    }
}

pub fn format_apply_report_json(
    run: &ReconcileRun,
    verification_results: &[VerificationResult],
) -> String {
    serde_json::json!({
        "run_id": run.run_id,
        "status": format!("{:?}", run.status).to_lowercase(),
        "summary": run.summary,
        "verification_results": verification_results.iter().map(|result| serde_json::json!({
            "target": result.target,
            "status": format!("{:?}", result.status).to_lowercase(),
            "details": result.details,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

fn quadlet_type_label(quadlet_type: Option<QuadletType>) -> &'static str {
    match quadlet_type {
        Some(QuadletType::Container) => "container",
        Some(QuadletType::Socket) => "socket",
        Some(QuadletType::SocketDropIn) => "socket-dropin",
        Some(QuadletType::ConfigFile) => "config",
        Some(QuadletType::Mount) => "mount",
        Some(QuadletType::Automount) => "automount",
        Some(QuadletType::Volume) => "volume",
        Some(QuadletType::Pod) => "pod",
        Some(QuadletType::Network) => "network",
        None => "unknown",
    }
}

fn action_label(
    action: &crate::core::types::PlanActionType,
    quadlet_type: Option<&QuadletType>,
) -> String {
    match action {
        crate::core::types::PlanActionType::PreparePath => "PreparePath".to_string(),
        crate::core::types::PlanActionType::WriteQuadlet => {
            if matches!(
                quadlet_type,
                Some(QuadletType::Socket)
                    | Some(QuadletType::SocketDropIn)
                    | Some(QuadletType::Mount)
                    | Some(QuadletType::Automount)
            ) {
                "WriteUnit".to_string()
            } else {
                "WriteQuadlet".to_string()
            }
        }
        crate::core::types::PlanActionType::RemoveQuadlet => {
            if matches!(
                quadlet_type,
                Some(QuadletType::Socket)
                    | Some(QuadletType::SocketDropIn)
                    | Some(QuadletType::Mount)
                    | Some(QuadletType::Automount)
            ) {
                "RemoveUnit".to_string()
            } else {
                "RemoveQuadlet".to_string()
            }
        }
        _ => format!("{:?}", action),
    }
}
