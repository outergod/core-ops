use crate::core::boundaries::enforce_plan_boundaries;
use crate::core::diff::diff_workloads;
use crate::core::errors::{CoreError, ValidationError};
use crate::core::types::{
    DependencyEdgeKind, DesiredState, DeterministicActionClass, DeterministicPlannedAction,
    DeterministicReconciliationPlan, DiffItem, DiffKind, FailureClass, GeneratedUnitSet,
    MountDeclaration, MountDependency, NormalizedSnapshot, PlanAction, PlanActionType,
    QuadletType, ReconciliationPlan, SafetyCheck, SemanticDependencyEdge, SemanticDependencyGraph,
    SemanticDependencyNode, ServiceDependencyEdit, StructuredDriftRecord, DriftCategory, ObservedState,
};
use crate::core::validation::validate_desired_state;
use std::collections::HashSet;
use std::path::Path;

pub fn plan(desired: &DesiredState, observed: &ObservedState) -> Result<ReconciliationPlan, CoreError> {
    validate_desired_state(desired).map_err(map_validation_error)?;

    let mut diffs = diff_workloads(&desired.workloads, &observed.workloads);
    order_diffs(&mut diffs);
    let mut actions = Vec::new();
    let prepared_paths = desired
        .mount_declarations
        .iter()
        .filter_map(|mount| {
            mount.prepared_path.as_ref().filter(|prepared| prepared.create_if_missing).map(|prepared| {
                (mount.mount_unit_name(), prepared.path.clone())
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let socket_stems = desired_socket_stems(&desired.workloads);
    let container_stems = desired_container_stems(&desired.workloads);
    let automount_stems = desired_automount_stems(&desired.workloads);
    for diff in &diffs {
        let quadlet_type = diff
            .desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|workload| workload.quadlet_type.clone());
        let mut diff_actions =
            actions_for_diff(
                diff.kind.clone(),
                &diff.name,
                quadlet_type,
                prepared_paths.get(&diff.name).map(String::as_str),
                &socket_stems,
                &container_stems,
                &automount_stems,
            );
        actions.append(&mut diff_actions);
    }

    let plan_id = format!(
        "plan:{}:{}",
        desired.revision_id,
        observed
            .observed_revision_id
            .clone()
            .unwrap_or_else(|| "none".to_string())
    );

    let plan = ReconciliationPlan {
        plan_id,
        desired_revision_id: desired.revision_id.clone(),
        observed_revision_id: observed.observed_revision_id.clone(),
        actions,
        safety_checks: vec![
            SafetyCheck::BoundariesDeclared,
            SafetyCheck::SupportedQuadletTypes,
            SafetyCheck::DeterministicPlan,
        ],
        expected_outcomes: vec!["observed state converges to desired state".to_string()],
    };

    enforce_plan_boundaries(&plan)?;
    Ok(plan)
}

pub fn build_semantic_dependency_graph(snapshot: &NormalizedSnapshot) -> SemanticDependencyGraph {
    let mut nodes: Vec<SemanticDependencyNode> = snapshot
        .objects
        .iter()
        .map(|object| SemanticDependencyNode {
            object_id: object.object_id.clone(),
            object_kind: object.object_kind.clone(),
            ordering_key: object.object_id.clone(),
        })
        .collect();
    nodes.sort_by(|a, b| a.ordering_key.cmp(&b.ordering_key));

    let mut edges = Vec::new();
    for object in &snapshot.objects {
        for dep in &object.dependency_refs {
            edges.push(SemanticDependencyEdge {
                from_object_id: dep.clone(),
                to_object_id: object.object_id.clone(),
                edge_kind: DependencyEdgeKind::Explicit,
                reason: "declared dependency".to_string(),
            });
        }
    }
    edges.sort_by(|a, b| {
        (&a.from_object_id, &a.to_object_id, &a.reason).cmp(&(&b.from_object_id, &b.to_object_id, &b.reason))
    });

    SemanticDependencyGraph { nodes, edges }
}

pub fn plan_deterministic_reconciliation(
    desired: &NormalizedSnapshot,
    last_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
) -> DeterministicReconciliationPlan {
    let graph = build_semantic_dependency_graph(desired);
    let mut desired_ids: Vec<String> = desired.objects.iter().map(|object| object.object_id.clone()).collect();
    desired_ids.sort();
    let actual_ids: HashSet<&str> = actual.objects.iter().map(|object| object.object_id.as_str()).collect();
    let applied_ids: HashSet<&str> = last_applied
        .map(|snapshot| snapshot.objects.iter().map(|object| object.object_id.as_str()).collect())
        .unwrap_or_default();

    let mut actions = Vec::new();
    let mut drift_records = Vec::new();

    for object_id in desired_ids {
        let classification = if !actual_ids.contains(object_id.as_str()) {
            DeterministicActionClass::Create
        } else if !applied_ids.is_empty() && !applied_ids.contains(object_id.as_str()) {
            DeterministicActionClass::Update
        } else {
            DeterministicActionClass::NoOp
        };
        if classification != DeterministicActionClass::NoOp {
            drift_records.push(StructuredDriftRecord {
                object_id: object_id.clone(),
                category: if classification == DeterministicActionClass::Create {
                    DriftCategory::ExpectedChange
                } else {
                    DriftCategory::ExternalDrift
                },
                comparison_basis: "three_way".to_string(),
                auto_action: true,
                attention_required: false,
                details: "deterministic planner scaffolding".to_string(),
            });
        }
        actions.push(DeterministicPlannedAction {
            object_id,
            classification,
            reason: "deterministic planner scaffolding".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: Default::default(),
        });
    }

    DeterministicReconciliationPlan {
        desired_revision_id: desired.revision_id.clone(),
        baseline_revision_id: last_applied.and_then(|snapshot| snapshot.revision_id.clone()),
        scope_id: desired.scope_id.clone(),
        actions,
        drift_records,
        graph,
    }
}

pub fn plan_mount_units(
    declaration: &MountDeclaration,
    dependencies: &[MountDependency],
) -> GeneratedUnitSet {
    let mount_unit_name = declaration.mount_unit_name();
    let automount_unit_name = declaration.automount_unit_name();
    let service_dependency_edits = dependencies
        .iter()
        .filter(|dependency| dependency.mount_ids.iter().any(|id| id == &declaration.id))
        .map(|dependency| ServiceDependencyEdit {
            service_name: dependency.service_name.clone(),
            requires_mounts_for: dependency.consumed_paths.clone(),
            after_units: explicit_dependency_units(declaration),
            requires_units: explicit_dependency_units(declaration),
        })
        .collect();

    let mut removal_candidates = vec![mount_unit_name.clone()];
    if let Some(unit) = &automount_unit_name {
        removal_candidates.insert(0, unit.clone());
    }

    GeneratedUnitSet {
        declaration_id: declaration.id.clone(),
        mount_unit_name,
        automount_unit_name,
        service_dependency_edits,
        removal_candidates,
    }
}

fn explicit_dependency_units(declaration: &MountDeclaration) -> Vec<String> {
    match declaration.automount_unit_name() {
        Some(automount_unit) => vec![automount_unit, declaration.mount_unit_name()],
        None => vec![declaration.mount_unit_name()],
    }
}

fn actions_for_diff(
    kind: DiffKind,
    name: &str,
    quadlet_type: Option<QuadletType>,
    prepared_path: Option<&str>,
    socket_stems: &HashSet<String>,
    container_stems: &HashSet<String>,
    automount_stems: &HashSet<String>,
) -> Vec<PlanAction> {
    let manage_unit = match quadlet_type {
        Some(QuadletType::SocketDropIn) => false,
        Some(QuadletType::ConfigFile) => false,
        Some(QuadletType::Volume) => false,
        Some(QuadletType::Mount | QuadletType::Automount) => true,
        Some(QuadletType::Container) => {
            let stem = stem_for_unit_name(name);
            match stem {
                Some(stem) => !socket_stems.contains(stem),
                None => true,
            }
        }
        _ => true,
    };
    let reload_systemd = !matches!(quadlet_type, Some(QuadletType::ConfigFile));
    let skip_mount_activation = matches!(quadlet_type, Some(QuadletType::Mount))
        && stem_for_unit_name(name)
            .map(|stem| automount_stems.contains(stem))
            .unwrap_or(false);
    match kind {
        DiffKind::Add => {
            let mut actions = Vec::new();
            if matches!(quadlet_type, Some(QuadletType::Mount)) {
                if let Some(path) = prepared_path {
                    actions.push(action(PlanActionType::PreparePath, path));
                }
            }
            actions.push(action(PlanActionType::WriteQuadlet, name));
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            if manage_unit && !skip_mount_activation {
                actions.push(action(PlanActionType::StartUnit, name));
            }
            if should_restart_socket_for_dropin(quadlet_type.as_ref(), name) {
                if let Some(socket_unit) = socket_unit_from_dropin_name(name) {
                    actions.push(action(PlanActionType::RestartUnit, &socket_unit));
                }
            }
            if should_start_service_for_socket(quadlet_type.as_ref(), name, container_stems) {
                actions.push(action(
                    PlanActionType::StartUnit,
                    &format!("{}.service", stem_for_unit_name(name).unwrap_or(name)),
                ));
            }
            actions
        }
        DiffKind::Remove => {
            let mut actions = Vec::new();
            if manage_unit {
                actions.push(action(PlanActionType::StopUnit, name));
            }
            actions.push(action(PlanActionType::RemoveQuadlet, name));
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            if should_restart_socket_for_dropin(quadlet_type.as_ref(), name) {
                if let Some(socket_unit) = socket_unit_from_dropin_name(name) {
                    actions.push(action(PlanActionType::RestartUnit, &socket_unit));
                }
            }
            actions
        }
        DiffKind::Change => {
            let mut actions = Vec::new();
            if matches!(quadlet_type, Some(QuadletType::Mount)) {
                if let Some(path) = prepared_path {
                    actions.push(action(PlanActionType::PreparePath, path));
                }
            }
            actions.push(action(PlanActionType::WriteQuadlet, name));
            if reload_systemd {
                actions.push(action(PlanActionType::ReloadSystemd, name));
            }
            if manage_unit && !skip_mount_activation {
                actions.push(action(PlanActionType::RestartUnit, name));
            }
            if should_restart_socket_for_dropin(quadlet_type.as_ref(), name) {
                if let Some(socket_unit) = socket_unit_from_dropin_name(name) {
                    actions.push(action(PlanActionType::RestartUnit, &socket_unit));
                }
            }
            if should_restart_service_for_container(quadlet_type.as_ref(), name, socket_stems) {
                actions.push(action(
                    PlanActionType::RestartUnit,
                    &format!("{}.service", stem_for_unit_name(name).unwrap_or(name)),
                ));
            }
            if should_start_service_for_socket(quadlet_type.as_ref(), name, container_stems) {
                actions.push(action(
                    PlanActionType::StartUnit,
                    &format!("{}.service", stem_for_unit_name(name).unwrap_or(name)),
                ));
            }
            actions
        }
    }
}

fn desired_socket_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Socket)
        .filter_map(|workload| stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string()))
        .collect()
}

fn desired_container_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Container)
        .filter_map(|workload| stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string()))
        .collect()
}

fn desired_automount_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Automount)
        .filter_map(|workload| stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string()))
        .collect()
}

fn stem_for_unit_name(name: &str) -> Option<&str> {
    Path::new(name).file_stem().and_then(|stem| stem.to_str())
}

fn should_start_service_for_socket(
    quadlet_type: Option<&QuadletType>,
    name: &str,
    container_stems: &HashSet<String>,
) -> bool {
    if !matches!(quadlet_type, Some(QuadletType::Socket)) {
        return false;
    }
    match stem_for_unit_name(name) {
        Some(stem) => container_stems.contains(stem),
        None => false,
    }
}

fn should_restart_service_for_container(
    quadlet_type: Option<&QuadletType>,
    name: &str,
    socket_stems: &HashSet<String>,
) -> bool {
    if !matches!(quadlet_type, Some(QuadletType::Container)) {
        return false;
    }
    match stem_for_unit_name(name) {
        Some(stem) => socket_stems.contains(stem),
        None => false,
    }
}

fn should_restart_socket_for_dropin(
    quadlet_type: Option<&QuadletType>,
    name: &str,
) -> bool {
    matches!(quadlet_type, Some(QuadletType::SocketDropIn))
        && socket_unit_from_dropin_name(name).is_some()
}

fn socket_unit_from_dropin_name(name: &str) -> Option<String> {
    let marker = ".socket.d/";
    name.find(marker).map(|idx| name[..idx + ".socket".len()].to_string())
}

fn order_diffs(diffs: &mut [DiffItem]) {
    diffs.sort_by(|a, b| {
        let a_key = ordering_key(a);
        let b_key = ordering_key(b);
        a_key.cmp(&b_key)
    });
}

fn ordering_key(diff: &DiffItem) -> (u8, String) {
    let quadlet_type = diff
        .desired
        .as_ref()
        .or(diff.observed.as_ref())
        .map(|w| w.quadlet_type.clone());

    let order = match diff.kind {
        DiffKind::Remove => reverse_order_for_type(quadlet_type),
        _ => order_for_type(quadlet_type),
    };

    (order, diff.name.clone())
}

fn order_for_type(quadlet_type: Option<QuadletType>) -> u8 {
    match quadlet_type {
        Some(QuadletType::ConfigFile) => 0,
        Some(QuadletType::Mount) => 1,
        Some(QuadletType::Automount) => 2,
        Some(QuadletType::Volume) => 3,
        Some(QuadletType::Network) => 4,
        Some(QuadletType::Container) => 5,
        Some(QuadletType::SocketDropIn) => 6,
        Some(QuadletType::Socket) => 7,
        Some(QuadletType::Pod) => 8,
        None => 9,
    }
}

fn reverse_order_for_type(quadlet_type: Option<QuadletType>) -> u8 {
    match quadlet_type {
        Some(QuadletType::SocketDropIn) => 0,
        Some(QuadletType::Socket) => 1,
        Some(QuadletType::Container) => 2,
        Some(QuadletType::Network) => 3,
        Some(QuadletType::Volume) => 4,
        Some(QuadletType::Automount) => 5,
        Some(QuadletType::Mount) => 6,
        Some(QuadletType::ConfigFile) => 7,
        Some(QuadletType::Pod) => 8,
        None => 9,
    }
}

fn action(action_type: PlanActionType, name: &str) -> PlanAction {
    PlanAction {
        action_type,
        target: name.to_string(),
        preconditions: Vec::new(),
        postconditions: Vec::new(),
    }
}

fn map_validation_error(err: ValidationError) -> CoreError {
    CoreError::new(FailureClass::Validation, err.message)
}
