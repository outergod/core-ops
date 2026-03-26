use crate::core::types::{
    DeterministicActionClass, DeterministicConvergenceRecord, DeterministicReconciliationPlan,
    DiffItem, QuadletType, ReconcileRun, ReconciliationPlan, RollbackTargetCandidate,
    VerificationResult,
};

pub fn format_plan_report(plan: &ReconciliationPlan, diffs: &[DiffItem]) -> String {
    let mut output = String::new();
    output.push_str(&format!("plan {} with {} actions\n", plan.plan_id, plan.actions.len()));
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
        "status": run_status_label(&run.status),
        "summary": run.summary,
        "verification_results": verification_results.iter().map(|result| serde_json::json!({
            "target": result.target,
            "status": verification_status_label(&result.status),
            "details": result.details,
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

pub fn format_convergence_report_json(
    run: &ReconcileRun,
    verification_results: &[VerificationResult],
    convergence: Option<&DeterministicConvergenceRecord>,
) -> String {
    let base = serde_json::json!({
        "run_id": run.run_id,
        "status": run_status_label(&run.status),
        "summary": run.summary,
        "verification_results": verification_results.iter().map(|result| serde_json::json!({
            "target": result.target,
            "status": verification_status_label(&result.status),
            "details": result.details,
        })).collect::<Vec<_>>(),
        "convergence": convergence.map(|record| serde_json::json!({
            "desired_revision_id": record.desired_revision_id,
            "scope_id": record.scope_id,
            "status": convergence_status_label(&record.status),
            "attempt_count": record.attempt_count,
            "affected_objects": record.affected_objects,
            "completed_actions": record.completed_actions,
            "failed_actions": record.failed_actions,
            "can_continue": record.can_continue,
        })),
    });
    base.to_string()
}

pub fn format_deterministic_plan_report(plan: &DeterministicReconciliationPlan) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "deterministic plan scope={} desired_revision={} baseline_revision={} actions={}\n",
        plan.scope_id,
        plan.desired_revision_id.as_deref().unwrap_or("none"),
        plan.baseline_revision_id.as_deref().unwrap_or("none"),
        plan.actions.len()
    ));
    output.push_str("actions\n");
    for action in &plan.actions {
        output.push_str(&format!(
            "- {}: {} [{}]\n",
            action_class_label(&action.classification),
            action.object_id,
            action.reason
        ));
        if !action.dependency_context.is_empty() {
            output.push_str(&format!(
                "  dependencies: {}\n",
                action.dependency_context.join(", ")
            ));
        }
        for (field, details) in &action.semantic_diff {
            output.push_str(&format!("  diff {} -> {}\n", field, details));
        }
    }
    output.push_str("drift\n");
    for record in &plan.drift_records {
        output.push_str(&format!(
            "- {}: {} [{} attention_required={}]\n",
            drift_category_label(&record.category),
            record.object_id,
            record.comparison_basis,
            record.attention_required
        ));
    }
    output
}

pub fn format_deterministic_plan_json(plan: &DeterministicReconciliationPlan) -> String {
    serde_json::json!({
        "scope_id": plan.scope_id,
        "desired_revision_id": plan.desired_revision_id,
        "baseline_revision_id": plan.baseline_revision_id,
        "actions": plan.actions.iter().map(|action| serde_json::json!({
            "object_id": action.object_id,
            "classification": action_class_label(&action.classification),
            "reason": action.reason,
            "dependency_context": action.dependency_context,
            "semantic_diff": action.semantic_diff,
        })).collect::<Vec<_>>(),
        "drift": plan.drift_records.iter().map(|record| serde_json::json!({
            "object_id": record.object_id,
            "category": drift_category_label(&record.category),
            "comparison_basis": record.comparison_basis,
            "auto_action": record.auto_action,
            "attention_required": record.attention_required,
            "details": record.details,
        })).collect::<Vec<_>>(),
        "graph": {
            "nodes": plan.graph.nodes.iter().map(|node| serde_json::json!({
                "object_id": node.object_id,
                "object_kind": object_kind_label(&node.object_kind),
                "ordering_key": node.ordering_key,
            })).collect::<Vec<_>>(),
            "edges": plan.graph.edges.iter().map(|edge| serde_json::json!({
                "from_object_id": edge.from_object_id,
                "to_object_id": edge.to_object_id,
                "edge_kind": edge_kind_label(&edge.edge_kind),
                "reason": edge.reason,
            })).collect::<Vec<_>>(),
        },
    })
    .to_string()
}

pub fn format_rollback_report(
    target: &RollbackTargetCandidate,
    plan: &DeterministicReconciliationPlan,
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "rollback target={} eligibility={:?}\n",
        target.target_revision_id, target.eligibility
    ));
    output.push_str(&format_deterministic_plan_report(plan));
    output
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

fn action_class_label(action: &DeterministicActionClass) -> &'static str {
    match action {
        DeterministicActionClass::Create => "create",
        DeterministicActionClass::Update => "update",
        DeterministicActionClass::Delete => "delete",
        DeterministicActionClass::Replace => "replace",
        DeterministicActionClass::NoOp => "no_op",
        DeterministicActionClass::Blocked => "blocked",
    }
}

fn object_kind_label(kind: &crate::core::types::ManagedObjectKind) -> &'static str {
    match kind {
        crate::core::types::ManagedObjectKind::GeneratedUnit => "generated_unit",
        crate::core::types::ManagedObjectKind::QuadletResource => "quadlet_resource",
        crate::core::types::ManagedObjectKind::Mount => "mount",
        crate::core::types::ManagedObjectKind::Automount => "automount",
        crate::core::types::ManagedObjectKind::RenderedArtifact => "rendered_artifact",
    }
}

fn edge_kind_label(kind: &crate::core::types::DependencyEdgeKind) -> &'static str {
    match kind {
        crate::core::types::DependencyEdgeKind::Explicit => "explicit",
        crate::core::types::DependencyEdgeKind::Implicit => "implicit",
    }
}

fn drift_category_label(category: &crate::core::types::DriftCategory) -> &'static str {
    match category {
        crate::core::types::DriftCategory::ExpectedChange => "expected_change",
        crate::core::types::DriftCategory::ExternalDrift => "external_drift",
        crate::core::types::DriftCategory::StaleResidue => "stale_residue",
        crate::core::types::DriftCategory::RuntimeVariance => "runtime_variance",
    }
}

fn run_status_label(status: &crate::core::types::RunStatus) -> &'static str {
    match status {
        crate::core::types::RunStatus::Success => "success",
        crate::core::types::RunStatus::Failure => "failure",
    }
}

fn verification_status_label(status: &crate::core::types::VerificationStatus) -> &'static str {
    match status {
        crate::core::types::VerificationStatus::Success => "success",
        crate::core::types::VerificationStatus::Failure => "failure",
    }
}

fn convergence_status_label(status: &crate::core::types::ConvergenceStatus) -> &'static str {
    match status {
        crate::core::types::ConvergenceStatus::Success => "success",
        crate::core::types::ConvergenceStatus::Partial => "partial",
        crate::core::types::ConvergenceStatus::Blocked => "blocked",
        crate::core::types::ConvergenceStatus::RepeatedFailure => "repeated_failure",
        crate::core::types::ConvergenceStatus::Oscillation => "oscillation",
        crate::core::types::ConvergenceStatus::Failed => "failed",
    }
}
