use crate::core::retry::{build_retry_observation, evaluate_retry_history, RetryObservation};
use crate::core::types::{
    ConvergenceStatus, DesiredState, DeterministicConvergenceRecord, MountDeclaration,
    ObservedState, QuadletType, UnitActiveState, VerificationResult, VerificationStatus, Workload,
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
            mount
                .automount_unit_name()
                .map(|automount_unit| (automount_unit, mount.clone()))
        })
        .collect();
    let socket_triggers = socket_trigger_map(&desired.workloads);
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
                &socket_triggers,
                observed,
            )
        })
        .collect()
}

/// Build a map from service unit name -> socket unit names that activate it.
///
/// A `.socket` unit's `Service=` directive (or, when absent, the default
/// `<stem>.service`) declares which service that socket activates on
/// connection. Multiple sockets can activate the same service — e.g. a Traefik
/// host with `http.socket`, `https.socket`, `traefik.socket` all targeting
/// `traefik.service`.
///
/// `Service=` is a single-valued directive: when systemd loads a base unit
/// plus its drop-ins, later assignments override earlier ones, and an empty
/// assignment resets the field to its default. Resolution here mirrors that
/// — base socket contents first, then `SocketDropIn` workloads sorted by
/// file name, taking the last non-empty assignment seen.
///
/// Used by `verify_workload` to recognise socket-activated services that are
/// correctly `Inactive` (no traffic yet) but whose listening sockets are
/// `Active`. Treating the service as failed in that state is wrong: systemd
/// will start it on first connection.
pub(crate) fn socket_trigger_map(workloads: &[Workload]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for workload in workloads {
        if workload.quadlet_type != QuadletType::Socket {
            continue;
        }
        let service = effective_socket_target_service(workload, workloads);
        map.entry(service)
            .or_default()
            .push(workload.systemd_unit_name.clone());
    }
    for entries in map.values_mut() {
        entries.sort();
        entries.dedup();
    }
    map
}

/// Resolve a socket's effective `Service=` target by walking the base socket
/// contents and every `SocketDropIn` workload that lives under
/// `<socket-unit-name>.d/`, sorted lex by file name. Last non-empty
/// assignment wins; an empty assignment (`Service=`) resets to the default
/// `<stem>.service`.
fn effective_socket_target_service(socket: &Workload, all: &[Workload]) -> String {
    let stem = Path::new(&socket.systemd_unit_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&socket.systemd_unit_name);
    let default_target = format!("{stem}.service");
    let mut current = default_target.clone();

    let dropin_prefix = format!("{}.d/", socket.systemd_unit_name);
    let mut dropins: Vec<&Workload> = all
        .iter()
        .filter(|w| {
            w.quadlet_type == QuadletType::SocketDropIn
                && w.systemd_unit_name.starts_with(&dropin_prefix)
        })
        .collect();
    dropins.sort_by(|a, b| a.systemd_unit_name.cmp(&b.systemd_unit_name));

    let sources = std::iter::once(socket.quadlet_contents.as_str())
        .chain(dropins.iter().map(|w| w.quadlet_contents.as_str()));

    for src in sources {
        for raw_line in src.lines() {
            let line = raw_line.trim_start();
            if line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some(value) = line
                .strip_prefix("Service=")
                .or_else(|| line.strip_prefix("service="))
            {
                let trimmed = value.trim();
                current = if trimmed.is_empty() {
                    default_target.clone()
                } else {
                    trimmed.to_string()
                };
            }
        }
    }
    current
}

pub fn evaluate_convergence(
    desired: &DesiredState,
    observed: &ObservedState,
    history: &[RetryObservation],
    retry_budget: u32,
) -> DeterministicConvergenceRecord {
    let verification_results = verify_state(desired, observed);
    let attempt = history.last().map(|entry| entry.attempt).unwrap_or(1);
    let evaluation = history
        .last()
        .cloned()
        .unwrap_or_else(|| build_retry_observation(attempt, &verification_results));
    let derived = evaluate_retry_history(history, retry_budget).unwrap_or_else(|| {
        evaluate_retry_history(&[evaluation], retry_budget).expect("derived convergence")
    });
    let failed_actions = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Failure)
        .map(|result| result.target.clone())
        .collect::<Vec<_>>();
    let completed_actions = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Success)
        .map(|result| result.target.clone())
        .collect::<Vec<_>>();

    let status = if verification_results
        .iter()
        .all(|result| result.status == VerificationStatus::Success)
    {
        ConvergenceStatus::Success
    } else {
        derived.status.clone()
    };
    let can_continue = matches!(
        status,
        ConvergenceStatus::Success | ConvergenceStatus::Partial | ConvergenceStatus::Failed
    );

    DeterministicConvergenceRecord {
        desired_revision_id: desired.revision_id.clone(),
        scope_id: observed
            .host_info
            .as_ref()
            .map(|host| format!("host:{}", host.hostname))
            .or_else(|| {
                std::env::var(crate::io::repo::HOST_OVERRIDE_ENV)
                    .ok()
                    .filter(|host| !host.is_empty())
                    .map(|host| format!("host:{host}"))
            })
            .or_else(default_host_scope_id)
            .unwrap_or_else(|| "scope:default".to_string()),
        status,
        attempt_count: attempt,
        affected_objects: if derived.affected_objects.is_empty() {
            failed_actions.clone()
        } else {
            derived.affected_objects.clone()
        },
        completed_actions,
        failed_actions,
        can_continue,
    }
}

fn default_host_scope_id() -> Option<String> {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if result != 0 {
        return None;
    }
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let hostname = String::from_utf8_lossy(&buf[..len]).trim().to_string();
    (!hostname.is_empty()).then(|| format!("host:{hostname}"))
}

fn verify_workload(
    quadlet_type: QuadletType,
    unit_file: &str,
    mount: Option<&MountDeclaration>,
    automount: Option<&MountDeclaration>,
    socket_triggers: &BTreeMap<String, Vec<String>>,
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
                return failure(
                    unit_name,
                    &format!("blocked: unit not active: {:?}", unit.active_state),
                );
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
        (QuadletType::Mount, None) => {
            let Some(mount) = mount else {
                return failure(unit_name, "mount declaration missing");
            };
            if mount.automount {
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
            failure(unit_name, "unit not found")
        }
        (QuadletType::Automount, Some(unit)) => {
            let _ = automount;
            if unit.active_state == UnitActiveState::Active {
                success(unit_name)
            } else {
                failure(
                    unit_name,
                    &format!("blocked: unit not active: {:?}", unit.active_state),
                )
            }
        }
        (_, Some(unit)) => {
            if unit.active_state == UnitActiveState::Active {
                return success(unit_name);
            }
            // Socket-activated services are correctly Inactive until first
            // connection. Accept Inactive when a triggering socket is Active —
            // systemd will start the service on demand. A Failed service is
            // never accepted, even with Active sockets, because that means the
            // service started and crashed.
            if unit.active_state == UnitActiveState::Inactive {
                if let Some(triggers) = socket_triggers.get(&unit_name) {
                    let any_socket_active = triggers.iter().any(|socket_unit| {
                        observed
                            .units
                            .iter()
                            .any(|u| u.unit_name == *socket_unit
                                && u.active_state == UnitActiveState::Active)
                    });
                    if any_socket_active {
                        return success(unit_name);
                    }
                }
            }
            failure(
                unit_name,
                &format!("unit not active: {:?}", unit.active_state),
            )
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
