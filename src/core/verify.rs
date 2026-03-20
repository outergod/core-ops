use crate::core::types::{
    DesiredState, ObservedState, QuadletType, UnitActiveState, VerificationResult,
    VerificationStatus,
};
use crate::core::unit::systemd_unit_for_quadlet_file;

pub fn verify_state(desired: &DesiredState, observed: &ObservedState) -> Vec<VerificationResult> {
    desired
        .workloads
        .iter()
        .map(|workload| verify_workload(workload.quadlet_type.clone(), &workload.systemd_unit_name, observed))
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
        (QuadletType::Volume, Some(_)) => VerificationResult {
            target: unit_name,
            status: VerificationStatus::Success,
            details: None,
        },
        (QuadletType::Volume, None) => VerificationResult {
            target: unit_name,
            status: VerificationStatus::Failure,
            details: Some("volume unit not found".to_string()),
        },
        (_, Some(unit)) => {
            if unit.active_state == UnitActiveState::Active {
                VerificationResult {
                    target: unit_name,
                    status: VerificationStatus::Success,
                    details: None,
                }
            } else {
                VerificationResult {
                    target: unit_name,
                    status: VerificationStatus::Failure,
                    details: Some(format!("unit not active: {:?}", unit.active_state)),
                }
            }
        }
        (_, None) => VerificationResult {
            target: unit_name,
            status: VerificationStatus::Failure,
            details: Some("unit not found".to_string()),
        },
    }
}
