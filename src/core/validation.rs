use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::core::errors::{ValidationError, ValidationErrorKind};
use crate::core::types::{
    ApplyOutputView, ArtifactSource, Boundaries, BoundaryScope, DesiredState, DropInSource,
    ExplainOutputView, HostDeclaration, Invariant, MountDeclaration, MountDependency,
    PathDependencyMode, PlanEntry, PlanOutputView, PreparedTargetPath, ResultOutputView,
    ServiceCatalog, UnitDependencyMode, Workload,
};

pub fn validate_desired_state(desired: &DesiredState) -> Result<(), ValidationError> {
    validate_invariants(&desired.invariants)?;
    validate_boundaries(&desired.boundaries)?;
    validate_workloads(&desired.workloads)?;
    validate_mount_model(
        &desired.mount_declarations,
        &desired.mount_dependencies,
        None,
    )?;
    Ok(())
}

pub fn validate_service_selection(
    host: &HostDeclaration,
    catalog: &ServiceCatalog,
) -> Result<(), ValidationError> {
    for service in &host.services {
        if !catalog.services.contains_key(service) {
            return Err(ValidationError::new(
                ValidationErrorKind::UndefinedServiceSelection,
                format!("undefined service selection: {}", service),
            ));
        }
    }
    Ok(())
}

pub fn validate_dropin_targets(
    dropins: &[DropInSource],
    artifacts: &[ArtifactSource],
) -> Result<(), ValidationError> {
    let targets: HashSet<String> = artifacts.iter().map(|a| a.name.clone()).collect();
    for dropin in dropins {
        if !targets.contains(&dropin.target) {
            return Err(ValidationError::new(
                ValidationErrorKind::MissingArtifactTarget,
                format!("drop-in target does not exist: {}", dropin.target),
            ));
        }
    }
    Ok(())
}

pub fn validate_socket_dropin_precedence(
    base_dropins: &[DropInSource],
    host_dropins: &[DropInSource],
) -> Result<(), ValidationError> {
    let mut base_max: HashMap<&str, String> = HashMap::new();
    for dropin in base_dropins {
        if !dropin.target.ends_with(".socket") {
            continue;
        }
        let file_name = dropin_file_name(&dropin.source_path);
        base_max
            .entry(dropin.target.as_str())
            .and_modify(|current| {
                if file_name > *current {
                    *current = file_name.clone();
                }
            })
            .or_insert(file_name);
    }

    for dropin in host_dropins {
        if !dropin.target.ends_with(".socket") {
            continue;
        }
        let Some(base_name) = base_max.get(dropin.target.as_str()) else {
            continue;
        };
        let file_name = dropin_file_name(&dropin.source_path);
        if file_name <= *base_name {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidDropInOrdering,
                format!(
                    "host socket drop-in must sort after base drop-ins: target={} host={} base_max={}",
                    dropin.target, file_name, base_name
                ),
            ));
        }
    }

    Ok(())
}

pub fn validate_config_paths(paths: &[String]) -> Result<(), ValidationError> {
    for path in paths {
        if !path.starts_with("/etc/") {
            return Err(ValidationError::new(
                ValidationErrorKind::MissingArtifactTarget,
                format!("config path outside allowed root: {}", path),
            ));
        }
        if path.contains("..") {
            return Err(ValidationError::new(
                ValidationErrorKind::MissingArtifactTarget,
                format!("config path traversal not allowed: {}", path),
            ));
        }
    }
    Ok(())
}

pub fn validate_mount_model(
    declarations: &[MountDeclaration],
    dependencies: &[MountDependency],
    selected_services: Option<&[String]>,
) -> Result<(), ValidationError> {
    let mut ids = HashSet::new();
    let mut targets = HashSet::new();
    let selected: HashSet<&str> = selected_services
        .map(|services| services.iter().map(String::as_str).collect())
        .unwrap_or_default();

    for declaration in declarations {
        if !ids.insert(declaration.id.as_str()) {
            return Err(ValidationError::new(
                ValidationErrorKind::DuplicateMountId,
                format!("duplicate mount declaration id: {}", declaration.id),
            ));
        }
        if !targets.insert(declaration.target_path.as_str()) {
            return Err(ValidationError::new(
                ValidationErrorKind::DuplicateMountTarget,
                format!("duplicate mount target path: {}", declaration.target_path),
            ));
        }
        validate_mount_declaration(declaration, &selected)?;
    }

    let declaration_map: HashMap<&str, &MountDeclaration> = declarations
        .iter()
        .map(|decl| (decl.id.as_str(), decl))
        .collect();

    for dependency in dependencies {
        validate_mount_dependency(dependency, &declaration_map, &selected)?;
    }

    Ok(())
}

pub fn validate_canonical_object_identity(object_id: &str) -> Result<(), ValidationError> {
    if object_id.trim().is_empty() {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            "object identity must not be empty",
        ));
    }
    if object_id.split_whitespace().count() > 1 {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            format!("object identity must not contain whitespace: {}", object_id),
        ));
    }
    Ok(())
}

pub fn detect_semantic_dependency_cycle(edges: &[(String, String)]) -> Result<(), ValidationError> {
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut nodes = HashSet::new();

    for (from, to) in edges {
        nodes.insert(from.as_str());
        nodes.insert(to.as_str());
        outgoing.entry(from.as_str()).or_default().push(to.as_str());
        *incoming.entry(to.as_str()).or_insert(0) += 1;
        incoming.entry(from.as_str()).or_insert(0);
    }

    let mut queue: Vec<&str> = nodes
        .iter()
        .copied()
        .filter(|node| incoming.get(node).copied().unwrap_or(0) == 0)
        .collect();
    let mut visited = 0usize;

    while let Some(node) = queue.pop() {
        visited += 1;
        if let Some(children) = outgoing.get(node) {
            for child in children {
                if let Some(count) = incoming.get_mut(child) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(child);
                    }
                }
            }
        }
    }

    if visited != nodes.len() {
        return Err(ValidationError::new(
            ValidationErrorKind::SemanticDependencyCycle,
            "semantic dependency cycle detected",
        ));
    }

    Ok(())
}

pub fn validate_rollback_candidate(
    available_revisions: &[String],
    scope_id: &str,
    candidate_revision: &str,
    candidate_scope: &str,
    retained: bool,
) -> Result<(), ValidationError> {
    if candidate_scope != scope_id {
        return Err(ValidationError::new(
            ValidationErrorKind::RollbackIneligible,
            format!(
                "rollback target scope mismatch: expected {} but got {}",
                scope_id, candidate_scope
            ),
        ));
    }
    if !retained
        || !available_revisions
            .iter()
            .any(|rev| rev == candidate_revision)
    {
        return Err(ValidationError::new(
            ValidationErrorKind::RollbackIneligible,
            format!("rollback target is not retained: {}", candidate_revision),
        ));
    }
    Ok(())
}

pub fn validate_retry_signature(signature: &str) -> Result<(), ValidationError> {
    if signature.trim().is_empty() || signature.split('|').count() < 2 {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidRetrySignature,
            format!("invalid retry signature: {}", signature),
        ));
    }
    Ok(())
}

pub fn validate_order_indices(entries: &[PlanEntry]) -> Result<(), ValidationError> {
    for (index, entry) in entries.iter().enumerate() {
        if entry.order_index != index {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                format!(
                    "plan entries must have sequential order indices: expected {} but got {}",
                    index, entry.order_index
                ),
            ));
        }
    }
    Ok(())
}

pub fn validate_plan_output_view(plan: &PlanOutputView) -> Result<(), ValidationError> {
    if plan.view_kind != "plan" {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            format!("unexpected view kind: {}", plan.view_kind),
        ));
    }
    validate_order_indices(&plan.entries)?;
    for entry in &plan.entries {
        validate_canonical_object_identity(&entry.object.display_id)?;
        if entry.object.name.trim().is_empty() || entry.object.resource_type.trim().is_empty() {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                "managed object references must include resource_type and name",
            ));
        }
        if !matches!(entry.unchanged, Some(true))
            && entry.action != crate::core::types::PlanEntryAction::NoOp
            && entry.causes.is_empty()
        {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                format!(
                    "non-no-op plan entries must include a cause: {}",
                    entry.object.display_id
                ),
            ));
        }
    }
    Ok(())
}

pub fn validate_apply_output_view(apply: &ApplyOutputView) -> Result<(), ValidationError> {
    if apply.view_kind != "apply" {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            format!("unexpected view kind: {}", apply.view_kind),
        ));
    }
    for (index, phase) in apply.phases.iter().enumerate() {
        if phase.sequence != index {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                format!(
                    "apply phases must have sequential order indices: expected {} but got {}",
                    index, phase.sequence
                ),
            ));
        }
    }
    let event_offset = apply.phases.len();
    for (index, event) in apply.events.iter().enumerate() {
        let expected = event_offset + index;
        if event.sequence != expected {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                format!(
                    "apply events must continue phase sequence ordering: expected {} but got {}",
                    expected, event.sequence
                ),
            ));
        }
        validate_canonical_object_identity(&event.object.display_id)?;
        if event.object.name.trim().is_empty() || event.object.resource_type.trim().is_empty() {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                "apply event objects must include resource_type and name",
            ));
        }
    }
    Ok(())
}

pub fn validate_result_output_view(result: &ResultOutputView) -> Result<(), ValidationError> {
    if result.view_kind != "result" {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            format!("unexpected view kind: {}", result.view_kind),
        ));
    }
    for entry in &result.entries {
        validate_canonical_object_identity(&entry.object.display_id)?;
        if entry.object.name.trim().is_empty() || entry.object.resource_type.trim().is_empty() {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidObjectIdentity,
                "result entry objects must include resource_type and name",
            ));
        }
    }
    Ok(())
}

pub fn validate_explain_output_view(explain: &ExplainOutputView) -> Result<(), ValidationError> {
    if explain.view_kind != "explain" {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            format!("unexpected view kind: {}", explain.view_kind),
        ));
    }
    validate_canonical_object_identity(&explain.object.display_id)?;
    if explain.object.name.trim().is_empty() || explain.object.resource_type.trim().is_empty() {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidObjectIdentity,
            "explain object must include resource_type and name",
        ));
    }
    Ok(())
}

fn validate_mount_declaration(
    declaration: &MountDeclaration,
    _selected_services: &HashSet<&str>,
) -> Result<(), ValidationError> {
    if !declaration.target_path.starts_with('/') || declaration.target_path.contains("..") {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidMountTarget,
            format!("invalid mount target path: {}", declaration.target_path),
        ));
    }
    if declaration.automount && !declaration.network_backed {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidAutomount,
            format!(
                "automount requires network-backed mount declaration: {}",
                declaration.id
            ),
        ));
    }
    if let Some(prepared) = &declaration.prepared_path {
        validate_prepared_target(prepared, declaration)?;
    }
    Ok(())
}

fn validate_prepared_target(
    prepared: &PreparedTargetPath,
    declaration: &MountDeclaration,
) -> Result<(), ValidationError> {
    if !prepared.path.starts_with('/') || prepared.path.contains("..") {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidPreparedPath,
            format!("invalid prepared target path: {}", prepared.path),
        ));
    }
    if prepared.path != declaration.target_path {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidPreparedPath,
            format!(
                "prepared target path must match mount target: {} != {}",
                prepared.path, declaration.target_path
            ),
        ));
    }
    Ok(())
}

fn validate_mount_dependency(
    dependency: &MountDependency,
    declarations: &HashMap<&str, &MountDeclaration>,
    selected_services: &HashSet<&str>,
) -> Result<(), ValidationError> {
    if !selected_services.is_empty()
        && !selected_services.contains(dependency.service_name.as_str())
    {
        return Err(ValidationError::new(
            ValidationErrorKind::InvalidMountOwnershipScope,
            format!(
                "mount dependency service outside selected services: {}",
                dependency.service_name
            ),
        ));
    }
    if dependency.path_dependency_mode != PathDependencyMode::RequiresMountsFor {
        return Err(ValidationError::new(
            ValidationErrorKind::ConflictingMountDefinition,
            format!(
                "unsupported path dependency mode for service {}",
                dependency.service_name
            ),
        ));
    }
    if dependency.unit_dependency_mode != UnitDependencyMode::AfterAndRequires {
        return Err(ValidationError::new(
            ValidationErrorKind::ConflictingMountDefinition,
            format!(
                "unsupported unit dependency mode for service {}",
                dependency.service_name
            ),
        ));
    }
    if dependency.mount_ids.is_empty() {
        return Err(ValidationError::new(
            ValidationErrorKind::MissingMountReference,
            format!("service {} declares no mount ids", dependency.service_name),
        ));
    }
    for mount_id in &dependency.mount_ids {
        declarations.get(mount_id.as_str()).ok_or_else(|| {
            ValidationError::new(
                ValidationErrorKind::MissingMountReference,
                format!(
                    "service {} references unknown mount declaration: {}",
                    dependency.service_name, mount_id
                ),
            )
        })?;
    }
    for path in &dependency.consumed_paths {
        if !path.starts_with('/') {
            return Err(ValidationError::new(
                ValidationErrorKind::InvalidMountTarget,
                format!("invalid consumed path: {}", path),
            ));
        }
    }
    Ok(())
}

fn dropin_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn validate_invariants(invariants: &[Invariant]) -> Result<(), ValidationError> {
    if !invariants.contains(&Invariant::BoundariesDeclared) {
        return Err(ValidationError::new(
            ValidationErrorKind::MissingInvariant,
            "missing invariant: BoundariesDeclared",
        ));
    }
    if !invariants.contains(&Invariant::DeterministicPlan) {
        return Err(ValidationError::new(
            ValidationErrorKind::MissingInvariant,
            "missing invariant: DeterministicPlan",
        ));
    }
    Ok(())
}

fn validate_boundaries(boundaries: &Boundaries) -> Result<(), ValidationError> {
    if !boundaries.has_scope(BoundaryScope::QuadletSystemd) {
        return Err(ValidationError::new(
            ValidationErrorKind::MissingBoundaryScope,
            "missing boundary scope: QuadletSystemd",
        ));
    }
    Ok(())
}

fn validate_workloads(workloads: &[Workload]) -> Result<(), ValidationError> {
    let mut unit_names = HashSet::new();

    for workload in workloads {
        if !unit_names.insert(workload.systemd_unit_name.clone()) {
            return Err(ValidationError::new(
                ValidationErrorKind::DuplicateUnitName,
                format!("duplicate unit name: {}", workload.systemd_unit_name),
            ));
        }
    }

    Ok(())
}
