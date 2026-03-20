use crate::core::types::{
    DesiredState, ObservedState, QuadletType, UnitActiveState, VerificationResult,
    VerificationStatus,
};
use crate::core::unit::systemd_unit_for_quadlet_file;

pub fn verify_state(desired: &DesiredState, observed: &ObservedState) -> Vec<VerificationResult> {
    desired
        .workloads
        .iter()
        .filter(|workload| workload.quadlet_type != QuadletType::SocketDropIn)
        .map(|workload| {
            verify_workload(
                workload.quadlet_type.clone(),
                &workload.systemd_unit_name,
                observed,
            )
        })
        .collect()
}

fn verify_workload(
    quadlet_type: QuadletType,
    unit_file: &str,
    observed: &ObservedState,
) -> VerificationResult {
    let unit_name = systemd_unit_for_quadlet_file(unit_file);
    let unit = observed
        .units
        .iter()
        .find(|unit| unit.unit_name == unit_name);

    match (quadlet_type, unit) {
        (QuadletType::Volume, Some(_)) => success(unit_name),
        (QuadletType::Volume, None) => failure(unit_name, "volume unit not found"),
        (_, Some(unit)) => {
            if unit.active_state == UnitActiveState::Active {
                success(unit_name)
            } else {
                failure(unit_name, &format!("unit not active: {:?}", unit.active_state))
            }
        }
        (_, None) => failure(unit_name, "unit not found"),
    }
}

fn success(target: String) -> VerificationResult {
    VerificationResult {
        target,
        status: VerificationStatus::Success,
        details: None,
    }
}

fn failure(target: String, details: &str) -> VerificationResult {
    VerificationResult {
        target,
        status: VerificationStatus::Failure,
        details: Some(details.to_string()),
    }
}
