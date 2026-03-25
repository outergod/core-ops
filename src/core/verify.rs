use crate::core::types::{
    DesiredState, MountDeclaration, ObservedState, QuadletType, UnitActiveState,
    VerificationResult, VerificationStatus,
};
use crate::core::unit::systemd_unit_for_quadlet_file;
use std::collections::BTreeMap;
use std::path::Path;

pub fn verify_state(desired: &DesiredState, observed: &ObservedState) -> Vec<VerificationResult> {
    let mount_map: BTreeMap<String, MountDeclaration> = desired
        .mount_declarations
        .iter()
        .map(|mount| (mount.mount_unit_name(), mount.clone()))
        .collect();
    let automount_map: BTreeMap<String, MountDeclaration> = desired
        .mount_declarations
        .iter()
        .filter_map(|mount| {
            mount.automount_unit_name()
                .map(|automount_unit| (automount_unit, mount.clone()))
        })
        .collect();
    desired
        .workloads
        .iter()
        .filter(|workload| {
            !matches!(
                workload.quadlet_type,
                QuadletType::SocketDropIn | QuadletType::ConfigFile
            )
        })
        .map(|workload| {
            verify_workload(
                workload.quadlet_type.clone(),
                &workload.systemd_unit_name,
                mount_map.get(&workload.systemd_unit_name),
                automount_map.get(&workload.systemd_unit_name),
                observed,
            )
        })
        .collect()
}

fn verify_workload(
    quadlet_type: QuadletType,
    unit_file: &str,
    mount: Option<&MountDeclaration>,
    automount: Option<&MountDeclaration>,
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
        (QuadletType::Mount, Some(unit)) => {
            if let Some(mount) = mount {
                if mount.automount && unit.active_state != UnitActiveState::Active {
                    let Some(automount_unit_name) = mount.automount_unit_name() else {
                        return failure(unit_name, "automount declaration missing");
                    };
                    let automount_unit = observed
                        .units
                        .iter()
                        .find(|unit| unit.unit_name == automount_unit_name);
                    return match automount_unit {
                        Some(automount_unit)
                            if automount_unit.active_state == UnitActiveState::Active =>
                        {
                            success(unit_name)
                        }
                        Some(automount_unit) => failure(
                            unit_name,
                            &format!(
                                "blocked: automount unit not active: {:?}",
                                automount_unit.active_state
                            ),
                        ),
                        None => failure(unit_name, "blocked: automount unit not found"),
                    };
                }
            }
            if unit.active_state != UnitActiveState::Active {
                return failure(unit_name, &format!("blocked: unit not active: {:?}", unit.active_state));
            }
            let Some(mount) = mount else {
                return failure(unit_name, "mount declaration missing");
            };
            if is_target_path_mounted(&mount.target_path) {
                success(unit_name)
            } else {
                failure(unit_name, "degraded: mount target not mounted")
            }
        }
        (QuadletType::Automount, Some(unit)) => {
            let _ = automount;
            if unit.active_state == UnitActiveState::Active {
                success(unit_name)
            } else {
                failure(unit_name, &format!("blocked: unit not active: {:?}", unit.active_state))
            }
        }
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

fn is_target_path_mounted(target_path: &str) -> bool {
    let mountinfo_path = std::env::var("CORE_OPS_MOUNTINFO_PATH")
        .unwrap_or_else(|_| "/proc/self/mountinfo".to_string());
    let Ok(contents) = std::fs::read_to_string(&mountinfo_path) else {
        return Path::new(target_path).exists();
    };
    contents.lines().any(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        fields.get(4).copied() == Some(target_path)
    })
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
