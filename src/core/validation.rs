use std::collections::HashSet;

use crate::core::errors::{ValidationError, ValidationErrorKind};
use crate::core::types::{
    ArtifactSource, Boundaries, BoundaryScope, DesiredState, DropInSource, HostDeclaration,
    Invariant, ServiceCatalog, Workload,
};

pub fn validate_desired_state(desired: &DesiredState) -> Result<(), ValidationError> {
    validate_invariants(&desired.invariants)?;
    validate_boundaries(&desired.boundaries)?;
    validate_workloads(&desired.workloads)?;
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
