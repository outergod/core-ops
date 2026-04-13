use crate::core::boundaries::enforce_plan_boundaries;
use crate::core::diff::{diff_normalized_snapshots, diff_workloads};
use crate::core::errors::{CoreError, ValidationError};
use crate::core::evaluate::dependency_refs_for_workload_state;
use crate::core::types::{
    DependencyEdgeKind, DependencyEdgeView, DependencyRelation, DesiredState,
    DeterministicActionClass, DeterministicPlannedAction, DeterministicReconciliationPlan,
    DiffItem, DiffKind, FailureClass, GeneratedUnitSet, ManagedObjectKind, ManagedObjectRef,
    MountDeclaration, MountDependency, NormalizedSnapshot, ObservedState, PlanAction,
    PlanActionType, QuadletType, ReconciliationPlan, SafetyCheck, SemanticDependencyEdge,
    SemanticDependencyGraph, SemanticDependencyNode, ServiceDependencyEdit, UnitActiveState,
    VerificationResult, VerificationStatus,
};
use crate::core::validation::{detect_semantic_dependency_cycle, validate_desired_state};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

pub fn plan(
    desired: &DesiredState,
    observed: &ObservedState,
) -> Result<ReconciliationPlan, CoreError> {
    validate_desired_state(desired).map_err(map_validation_error)?;

    let mut diffs = diff_workloads(&desired.workloads, &observed.workloads);
    order_diffs(&mut diffs);
    let mut actions = Vec::new();
    let prepared_paths = desired
        .mount_declarations
        .iter()
        .filter_map(|mount| {
            mount
                .prepared_path
                .as_ref()
                .filter(|prepared| prepared.create_if_missing)
                .map(|prepared| (mount.mount_unit_name(), prepared.path.clone()))
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
        let mut diff_actions = actions_for_diff(
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

    // Dependent-restart pass: schedule RestartUnit for containers whose config
    // files changed, were added (when container already running), or were removed.
    let config_diffs: Vec<(&DiffKind, &str)> = diffs
        .iter()
        .filter(|diff| {
            matches!(diff.kind, DiffKind::Add | DiffKind::Change | DiffKind::Remove)
                && diff
                    .desired
                    .as_ref()
                    .or(diff.observed.as_ref())
                    .map(|w| w.quadlet_type == QuadletType::ConfigFile)
                    .unwrap_or(false)
        })
        .map(|diff| (&diff.kind, diff.name.as_str()))
        .collect();

    if !config_diffs.is_empty() {
        // For Add: only restart containers that are currently active (running).
        // Keying off observed.units (runtime state) prevents RestartUnit from
        // unintentionally starting services that were intentionally stopped.
        let observed_active_units: HashSet<&str> = observed
            .units
            .iter()
            .filter(|u| u.active_state == UnitActiveState::Active)
            .map(|u| u.unit_name.as_str())
            .collect();
        let mut already_restarted: HashSet<String> = actions
            .iter()
            .filter(|a| a.action_type == PlanActionType::RestartUnit)
            .map(|a| a.target.clone())
            .collect();

        for (kind, config_name) in &config_diffs {
            // For Remove: the config path is absent from desired.managed_config_paths,
            // so dependency_refs_for_workload_state would miss it. Augment a clone of
            // desired with the removed path so the full dependency parser (EnvironmentFile=,
            // Volume= roots, etc.) can resolve the dependency correctly.
            let augmented_desired;
            let effective_desired = if matches!(kind, DiffKind::Remove) {
                augmented_desired = {
                    let mut d = desired.clone();
                    d.managed_config_paths.push(config_name.to_string());
                    d
                };
                &augmented_desired
            } else {
                desired
            };

            for workload in &desired.workloads {
                let depends = dependency_refs_for_workload_state(effective_desired, workload)
                    .contains(&config_name.to_string());

                if depends {
                    let should_restart = match kind {
                        DiffKind::Add => observed_active_units
                            .contains(workload.systemd_unit_name.as_str()),
                        _ => true,
                    };
                    if should_restart && !already_restarted.contains(&workload.systemd_unit_name) {
                        actions.push(action(
                            PlanActionType::RestartUnit,
                            &workload.systemd_unit_name,
                        ));
                        already_restarted.insert(workload.systemd_unit_name.clone());
                    }
                }
            }
        }
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
        (&a.from_object_id, &a.to_object_id, &a.reason).cmp(&(
            &b.from_object_id,
            &b.to_object_id,
            &b.reason,
        ))
    });

    SemanticDependencyGraph { nodes, edges }
}

pub fn managed_object_ref(object_id: &str, object_kind: &ManagedObjectKind) -> ManagedObjectRef {
    let name = object_id.to_string();
    let resource_type = resource_type_for_object(object_id, object_kind).to_string();
    let display_name = if let Some(path) = object_id.strip_prefix("config:") {
        path.trim_start_matches('/').to_string()
    } else if object_id.starts_with('/') {
        object_id.trim_start_matches('/').to_string()
    } else {
        object_id.to_string()
    };
    ManagedObjectRef {
        display_id: format!("{resource_type}/{display_name}"),
        resource_type,
        name,
    }
}

fn resource_type_for_object(object_id: &str, object_kind: &ManagedObjectKind) -> &'static str {
    match object_kind {
        ManagedObjectKind::RenderedArtifact => "config",
        ManagedObjectKind::Mount => "mount",
        ManagedObjectKind::Automount => "automount",
        ManagedObjectKind::GeneratedUnit => "service",
        ManagedObjectKind::QuadletResource => {
            if object_id.ends_with(".container") {
                "container"
            } else if object_id.ends_with(".volume") {
                "volume"
            } else if object_id.ends_with(".network") {
                "network"
            } else if object_id.ends_with(".socket") {
                "socket"
            } else if object_id.ends_with(".mount") {
                "mount"
            } else if object_id.ends_with(".automount") {
                "automount"
            } else if object_id.ends_with(".service") {
                "service"
            } else if object_id.starts_with('/') {
                "config"
            } else {
                "resource"
            }
        }
    }
}

pub fn object_kind_by_id(graph: &SemanticDependencyGraph) -> BTreeMap<&str, &ManagedObjectKind> {
    graph
        .nodes
        .iter()
        .map(|node| (node.object_id.as_str(), &node.object_kind))
        .collect()
}

pub fn direct_prerequisite_refs(
    graph: &SemanticDependencyGraph,
    object_id: &str,
) -> Vec<ManagedObjectRef> {
    let object_kinds = object_kind_by_id(graph);
    let mut refs = graph
        .edges
        .iter()
        .filter(|edge| edge.to_object_id == object_id)
        .filter_map(|edge| {
            object_kinds
                .get(edge.from_object_id.as_str())
                .map(|kind| managed_object_ref(&edge.from_object_id, kind))
        })
        .collect::<Vec<_>>();
    refs.sort_by(|a, b| a.display_id.cmp(&b.display_id));
    refs
}

pub fn dependent_refs(graph: &SemanticDependencyGraph, object_id: &str) -> Vec<ManagedObjectRef> {
    let object_kinds = object_kind_by_id(graph);
    let mut refs = graph
        .edges
        .iter()
        .filter(|edge| edge.from_object_id == object_id)
        .filter_map(|edge| {
            object_kinds
                .get(edge.to_object_id.as_str())
                .map(|kind| managed_object_ref(&edge.to_object_id, kind))
        })
        .collect::<Vec<_>>();
    refs.sort_by(|a, b| a.display_id.cmp(&b.display_id));
    refs
}

pub fn direct_and_transitive_prerequisite_refs(
    graph: &SemanticDependencyGraph,
    object_id: &str,
) -> (Vec<ManagedObjectRef>, Vec<ManagedObjectRef>) {
    let direct = direct_prerequisite_refs(graph, object_id);
    let object_kinds = object_kind_by_id(graph);
    let mut seen = BTreeSet::new();
    let mut pending = graph
        .edges
        .iter()
        .filter(|edge| edge.to_object_id == object_id)
        .map(|edge| edge.from_object_id.clone())
        .collect::<Vec<_>>();
    let direct_ids = direct
        .iter()
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let mut transitive = Vec::new();

    while let Some(current) = pending.pop() {
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.to_object_id == current)
        {
            if seen.insert(edge.from_object_id.clone())
                && !direct_ids.contains(&edge.from_object_id)
            {
                if let Some(kind) = object_kinds.get(edge.from_object_id.as_str()) {
                    transitive.push(managed_object_ref(&edge.from_object_id, kind));
                }
                pending.push(edge.from_object_id.clone());
            }
        }
    }

    transitive.sort_by(|a, b| a.display_id.cmp(&b.display_id));
    (direct, transitive)
}

pub fn dependency_edges_for_object(
    graph: &SemanticDependencyGraph,
    object_id: &str,
) -> Vec<DependencyEdgeView> {
    direct_prerequisite_refs(graph, object_id)
        .into_iter()
        .map(|object| DependencyEdgeView {
            relation: DependencyRelation::Prerequisite,
            object,
        })
        .collect()
}

pub fn plan_deterministic_reconciliation(
    desired: &NormalizedSnapshot,
    last_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
) -> Result<DeterministicReconciliationPlan, CoreError> {
    plan_deterministic_reconciliation_with_runtime(desired, last_applied, actual, &[])
}

pub fn plan_deterministic_reconciliation_with_runtime(
    desired: &NormalizedSnapshot,
    last_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
    verification_results: &[VerificationResult],
) -> Result<DeterministicReconciliationPlan, CoreError> {
    let graph = build_semantic_dependency_graph(desired);
    let graph_edges = graph
        .edges
        .iter()
        .map(|edge| (edge.from_object_id.clone(), edge.to_object_id.clone()))
        .collect::<Vec<_>>();
    detect_semantic_dependency_cycle(&graph_edges).map_err(map_validation_error)?;
    let mut drift_records = diff_normalized_snapshots(desired, last_applied, actual);
    let desired_map = index_normalized_objects(&desired.objects);
    let actual_map = index_normalized_objects(&actual.objects);
    let applied_map = last_applied
        .map(|snapshot| index_normalized_objects(&snapshot.objects))
        .unwrap_or_default();
    let runtime_variance_by_object = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Failure)
        .map(|result| (result.target.as_str(), result.details.as_deref()))
        .collect::<BTreeMap<_, _>>();
    let desired_ids = ordered_desired_ids(&graph);
    let mut actions = Vec::new();
    for object_id in desired_ids {
        let desired_object = desired_map
            .get(object_id.as_str())
            .expect("desired object is present");
        let dependency_context = desired_object.dependency_refs.clone();
        let actual_object = actual_map.get(object_id.as_str());
        let applied_object = applied_map.get(object_id.as_str());
        let runtime_variance = runtime_variance_by_object
            .get(object_id.as_str())
            .copied()
            .flatten();
        let classification = if dependency_context
            .iter()
            .any(|dependency| !desired_map.contains_key(dependency.as_str()))
        {
            DeterministicActionClass::Blocked
        } else if actual_object.is_none() {
            DeterministicActionClass::Create
        } else if actual_object != Some(desired_object) {
            DeterministicActionClass::Update
        } else if runtime_variance.is_some() {
            DeterministicActionClass::Recover
        } else {
            DeterministicActionClass::NoOp
        };
        actions.push(DeterministicPlannedAction {
            object_id: object_id.clone(),
            classification,
            reason: action_reason(
                desired_object,
                applied_object,
                actual_object,
                &dependency_context,
                runtime_variance,
            ),
            dependency_context,
            semantic_diff: semantic_diff(desired_object, actual_object, applied_object),
        });
    }

    let mut changed_by_object = actions
        .iter()
        .map(|action| (action.object_id.clone(), action.classification.clone()))
        .collect::<BTreeMap<_, _>>();
    for action in &mut actions {
        if action.classification != DeterministicActionClass::NoOp {
            continue;
        }
        if let Some(trigger) =
            restart_trigger_dependency(&action.dependency_context, &changed_by_object)
        {
            action.classification = DeterministicActionClass::Restart;
            action.reason = format!("restart required because {} changed", trigger);
            changed_by_object.insert(action.object_id.clone(), DeterministicActionClass::Restart);
        }
    }

    for object_id in ordered_delete_ids(actual, desired).map_err(map_validation_error)? {
        actions.push(DeterministicPlannedAction {
            object_id,
            classification: DeterministicActionClass::Delete,
            reason: "actual object is outside desired snapshot".to_string(),
            dependency_context: Vec::new(),
            semantic_diff: BTreeMap::new(),
        });
    }

    for action in &actions {
        if action.classification != DeterministicActionClass::Recover {
            continue;
        }
        if drift_records
            .iter()
            .any(|record| record.object_id == action.object_id)
        {
            continue;
        }
        drift_records.push(crate::core::types::StructuredDriftRecord {
            object_id: action.object_id.clone(),
            category: crate::core::types::DriftCategory::RuntimeVariance,
            comparison_basis: "runtime_verification".to_string(),
            auto_action: true,
            attention_required: true,
            details: action.reason.clone(),
        });
    }
    drift_records.sort_by(|a, b| a.object_id.cmp(&b.object_id));

    Ok(DeterministicReconciliationPlan {
        desired_revision_id: desired.revision_id.clone(),
        baseline_revision_id: last_applied.and_then(|snapshot| snapshot.revision_id.clone()),
        requested_repository: None,
        requested_ref: None,
        last_applied_requested_repository: None,
        last_applied_requested_ref: None,
        scope_id: desired.scope_id.clone(),
        actions,
        drift_records,
        graph,
    })
}

fn restart_trigger_dependency(
    dependency_context: &[String],
    changed_by_object: &BTreeMap<String, DeterministicActionClass>,
) -> Option<String> {
    dependency_context.iter().find_map(|dependency| {
        changed_by_object
            .get(dependency)
            .and_then(|classification| {
                matches!(
                    classification,
                    DeterministicActionClass::Create
                        | DeterministicActionClass::Update
                        | DeterministicActionClass::Replace
                        | DeterministicActionClass::Restart
                )
                .then(|| dependency.clone())
            })
    })
}

pub fn plan_rollback_reconciliation(
    target_snapshot: &NormalizedSnapshot,
    current_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
    target_revision_id: &str,
) -> Result<DeterministicReconciliationPlan, CoreError> {
    let mut plan = plan_deterministic_reconciliation(target_snapshot, current_applied, actual)?;
    for action in &mut plan.actions {
        action.reason = match action.classification {
            DeterministicActionClass::Delete | DeterministicActionClass::Replace => format!(
                "rollback to {} requires disruptive {}",
                target_revision_id,
                action_class_label(&action.classification)
            ),
            _ => format!("rollback to {}: {}", target_revision_id, action.reason),
        };
    }
    Ok(plan)
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

fn index_normalized_objects(
    objects: &[crate::core::types::NormalizedManagedObject],
) -> BTreeMap<&str, &crate::core::types::NormalizedManagedObject> {
    let mut map = BTreeMap::new();
    for object in objects {
        map.insert(object.object_id.as_str(), object);
    }
    map
}

fn ordered_desired_ids(graph: &SemanticDependencyGraph) -> Vec<String> {
    let mut incoming = BTreeMap::<&str, usize>::new();
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
    for node in &graph.nodes {
        incoming.insert(node.object_id.as_str(), 0);
        outgoing.insert(node.object_id.as_str(), Vec::new());
    }
    for edge in &graph.edges {
        if incoming.contains_key(edge.to_object_id.as_str())
            && outgoing.contains_key(edge.from_object_id.as_str())
        {
            *incoming
                .get_mut(edge.to_object_id.as_str())
                .expect("incoming edge") += 1;
            outgoing
                .get_mut(edge.from_object_id.as_str())
                .expect("outgoing edge")
                .push(edge.to_object_id.as_str());
        }
    }

    let mut ready = BTreeSet::new();
    for node in &graph.nodes {
        if incoming.get(node.object_id.as_str()) == Some(&0) {
            ready.insert(node.ordering_key.clone());
        }
    }

    let mut ordered = Vec::new();
    while let Some(next) = ready.pop_first() {
        ordered.push(next.clone());
        let neighbors = outgoing.get(next.as_str()).cloned().unwrap_or_default();
        for neighbor in neighbors {
            let count = incoming.get_mut(neighbor).expect("neighbor present");
            *count -= 1;
            if *count == 0 {
                ready.insert(neighbor.to_string());
            }
        }
    }

    ordered
}

fn action_reason(
    desired: &crate::core::types::NormalizedManagedObject,
    applied: Option<&&crate::core::types::NormalizedManagedObject>,
    actual: Option<&&crate::core::types::NormalizedManagedObject>,
    dependency_context: &[String],
    runtime_variance: Option<&str>,
) -> String {
    if dependency_context
        .iter()
        .any(|dependency| dependency == &desired.object_id)
    {
        "object declares a self-dependency".to_string()
    } else if let Some(details) = runtime_variance {
        format!("runtime reconciliation required: {details}")
    } else if dependency_context.is_empty() {
        action_reason_without_dependencies(desired, applied, actual)
    } else if actual.is_none() {
        "object missing from actual state after dependency prerequisites".to_string()
    } else if actual != Some(&desired) {
        "actual state diverged from desired snapshot after dependency ordering".to_string()
    } else if applied != Some(&desired) {
        "desired snapshot changed since last applied state but actual already converged".to_string()
    } else {
        "desired, last applied, and actual state already match after dependency ordering"
            .to_string()
    }
}

fn action_reason_without_dependencies(
    desired: &crate::core::types::NormalizedManagedObject,
    applied: Option<&&crate::core::types::NormalizedManagedObject>,
    actual: Option<&&crate::core::types::NormalizedManagedObject>,
) -> String {
    if actual.is_none() {
        "object missing from actual state".to_string()
    } else if actual != Some(&desired) {
        "actual state diverged from desired snapshot".to_string()
    } else if applied != Some(&desired) {
        "desired snapshot changed since last applied state but actual already converged".to_string()
    } else {
        "desired, last applied, and actual state already match".to_string()
    }
}

fn semantic_diff(
    desired: &crate::core::types::NormalizedManagedObject,
    actual: Option<&&crate::core::types::NormalizedManagedObject>,
    applied: Option<&&crate::core::types::NormalizedManagedObject>,
) -> BTreeMap<String, String> {
    if actual == Some(&desired) {
        return BTreeMap::new();
    }
    let mut diff = BTreeMap::new();
    for (key, desired_value) in &desired.material_fields {
        let actual_value = actual.and_then(|object| object.material_fields.get(key));
        let applied_value = applied.and_then(|object| object.material_fields.get(key));
        if actual_value != Some(desired_value) || applied_value != Some(desired_value) {
            diff.insert(
                key.clone(),
                format!(
                    "desired={} actual={} applied={}",
                    desired_value,
                    actual_value.map(String::as_str).unwrap_or("<absent>"),
                    applied_value.map(String::as_str).unwrap_or("<absent>"),
                ),
            );
        }
    }
    diff
}

pub fn ordered_delete_ids(
    actual: &NormalizedSnapshot,
    desired: &NormalizedSnapshot,
) -> Result<Vec<String>, ValidationError> {
    let actual_graph = build_semantic_dependency_graph(actual);
    let graph_edges = actual_graph
        .edges
        .iter()
        .map(|edge| (edge.from_object_id.clone(), edge.to_object_id.clone()))
        .collect::<Vec<_>>();
    detect_semantic_dependency_cycle(&graph_edges)?;

    let desired_ids = desired
        .objects
        .iter()
        .map(|object| object.object_id.as_str())
        .collect::<HashSet<_>>();
    let mut stale = ordered_desired_ids(&actual_graph)
        .into_iter()
        .filter(|object_id| !desired_ids.contains(object_id.as_str()))
        .collect::<Vec<_>>();
    stale.reverse();
    Ok(stale)
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

fn action_class_label(action: &DeterministicActionClass) -> &'static str {
    match action {
        DeterministicActionClass::Create => "create",
        DeterministicActionClass::Update => "update",
        DeterministicActionClass::Delete => "delete",
        DeterministicActionClass::Replace => "replace",
        DeterministicActionClass::Recover => "recover",
        DeterministicActionClass::Restart => "restart",
        DeterministicActionClass::NoOp => "no_op",
        DeterministicActionClass::Blocked => "blocked",
    }
}

fn desired_socket_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Socket)
        .filter_map(|workload| {
            stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string())
        })
        .collect()
}

fn desired_container_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Container)
        .filter_map(|workload| {
            stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string())
        })
        .collect()
}

fn desired_automount_stems(workloads: &[crate::core::types::Workload]) -> HashSet<String> {
    workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::Automount)
        .filter_map(|workload| {
            stem_for_unit_name(&workload.systemd_unit_name).map(|s| s.to_string())
        })
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

fn should_restart_socket_for_dropin(quadlet_type: Option<&QuadletType>, name: &str) -> bool {
    matches!(quadlet_type, Some(QuadletType::SocketDropIn))
        && socket_unit_from_dropin_name(name).is_some()
}

fn socket_unit_from_dropin_name(name: &str) -> Option<String> {
    let marker = ".socket.d/";
    name.find(marker)
        .map(|idx| name[..idx + ".socket".len()].to_string())
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
