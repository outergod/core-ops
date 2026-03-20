use std::collections::HashSet;

use crate::core::errors::{ValidationError, ValidationErrorKind};
use crate::core::types::{Boundaries, BoundaryScope, DesiredState, Invariant, Workload};

pub fn validate_desired_state(desired: &DesiredState) -> Result<(), ValidationError> {
    validate_invariants(&desired.invariants)?;
    validate_boundaries(&desired.boundaries)?;
    validate_workloads(&desired.workloads)?;
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
