use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::types::{
    DesiredState, EnabledState, ObservedState, ObservedUnit, QuadletType, RestartPolicy,
    UnitActiveState, Workload,
};
use crate::io::quadlet::{
    normalize_socket_contents, parse_quadlet_name, read_quadlet_dir, QuadletError,
    SOCKET_MANAGED_MARKER,
};
use crate::io::systemd::systemd_unit_dir;
use crate::core::unit::systemd_unit_for_quadlet_file;

#[derive(Debug)]
pub enum ObservedError {
    MissingQuadletDir(PathBuf),
    SystemdQueryFailed(String),
    Quadlet(QuadletError),
    Io(std::io::Error),
}

impl From<QuadletError> for ObservedError {
    fn from(err: QuadletError) -> Self {
        ObservedError::Quadlet(err)
    }
}

impl From<std::io::Error> for ObservedError {
    fn from(err: std::io::Error) -> Self {
        ObservedError::Io(err)
    }
}

impl std::fmt::Display for ObservedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservedError::MissingQuadletDir(path) => {
                write!(f, "missing quadlet dir: {}", path.display())
            }
            ObservedError::SystemdQueryFailed(msg) => write!(f, "systemd query failed: {}", msg),
            ObservedError::Quadlet(err) => write!(f, "observed state error: {}", err),
            ObservedError::Io(err) => write!(f, "observed state io error: {}", err),
        }
    }
}

impl std::error::Error for ObservedError {}

pub fn read_observed_state(
    quadlet_dir: &Path,
    desired: Option<&DesiredState>,
    observed_revision_id: Option<String>,
) -> Result<ObservedState, ObservedError> {
    if !quadlet_dir.exists() {
        return Err(ObservedError::MissingQuadletDir(quadlet_dir.to_path_buf()));
    }

    let mut workloads: Vec<Workload> = read_quadlet_dir(quadlet_dir)?;
    let socket_dir = systemd_unit_dir();
    let socket_units = read_socket_units(&socket_dir)?;
    let allowed_socket_dropins = desired
        .map(desired_socket_dropins)
        .unwrap_or_default();
    let socket_dropins = read_socket_dropins(&socket_dir, &socket_units, &allowed_socket_dropins)?;
    workloads.extend(socket_units);
    workloads.extend(socket_dropins);
    if let Some(desired) = desired {
        workloads.extend(read_config_files(&desired.managed_config_roots)?);
    }
    let units = read_systemd_units(&workloads)?;

    Ok(ObservedState {
        observed_revision_id,
        units,
        workloads,
        last_reconcile_id: None,
        host_info: None,
    })
}

fn read_socket_units(dir: &Path) -> Result<Vec<Workload>, ObservedError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut workloads = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if !file_name.ends_with(".socket") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)?;
        if !contents.contains(SOCKET_MANAGED_MARKER) {
            continue;
        }
        let (name, quadlet_type) = parse_quadlet_name(file_name)?;
        if quadlet_type != QuadletType::Socket {
            continue;
        }
        let normalized = normalize_socket_contents(&contents);
        workloads.push(Workload {
            name,
            quadlet_type,
            quadlet_contents: normalized,
            systemd_unit_name: file_name.to_string(),
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        });
    }

    Ok(workloads)
}

fn read_socket_dropins(
    dir: &Path,
    sockets: &[Workload],
    allowed_dropins: &HashSet<String>,
) -> Result<Vec<Workload>, ObservedError> {
    if allowed_dropins.is_empty() {
        return Ok(Vec::new());
    }
    let mut workloads = Vec::new();
    for socket in sockets {
        let dropin_dir = dir.join(format!("{}.d", socket.systemd_unit_name));
        if !dropin_dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(dropin_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                continue;
            }
            let file_name = match path.file_name().and_then(|name| name.to_str()) {
                Some(name) if !name.starts_with('.') => name.to_string(),
                _ => continue,
            };
            if !file_name.ends_with(".conf") {
                continue;
            }
            let contents = std::fs::read_to_string(&path)?;
            let unit_name = format!("{}.d/{}", socket.systemd_unit_name, file_name);
            if !allowed_dropins.contains(&unit_name) {
                continue;
            }
            workloads.push(Workload {
                name: unit_name.clone(),
                quadlet_type: QuadletType::SocketDropIn,
                quadlet_contents: contents,
                systemd_unit_name: unit_name,
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            });
        }
    }
    Ok(workloads)
}

fn desired_socket_dropins(desired: &DesiredState) -> HashSet<String> {
    desired
        .workloads
        .iter()
        .filter(|workload| workload.quadlet_type == QuadletType::SocketDropIn)
        .map(|workload| workload.systemd_unit_name.clone())
        .collect()
}

fn read_systemd_units(workloads: &[Workload]) -> Result<Vec<ObservedUnit>, ObservedError> {
    if !systemctl_available() {
        log::warn!("systemctl unavailable; skipping unit discovery");
        return Ok(Vec::new());
    }

    let mut units = Vec::new();
    for workload in workloads {
        if matches!(workload.quadlet_type, QuadletType::SocketDropIn | QuadletType::ConfigFile) {
            continue;
        }
        let unit_name = systemd_unit_for_quadlet_file(&workload.systemd_unit_name);
        match query_unit_state(&unit_name)? {
            Some(unit) => units.push(unit),
            None => {}
        }
    }

    Ok(units)
}

fn read_config_files(paths: &[String]) -> Result<Vec<Workload>, ObservedError> {
    let mut workloads = Vec::new();
    for config_path in paths {
        let path = Path::new(config_path);
        if !path.exists() {
            continue;
        }
        if path.is_dir() {
            read_config_dir(path, &mut workloads)?;
        } else if path.is_file() {
            let contents = std::fs::read_to_string(path)?;
            workloads.push(Workload {
                name: config_path.clone(),
                quadlet_type: QuadletType::ConfigFile,
                quadlet_contents: contents,
                systemd_unit_name: config_path.clone(),
                enabled_state: EnabledState::Enabled,
                restart_policy: RestartPolicy::Always,
            });
        }
    }
    Ok(workloads)
}

fn read_config_dir(dir: &Path, workloads: &mut Vec<Workload>) -> Result<(), ObservedError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            read_config_dir(&path, workloads)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let contents = std::fs::read_to_string(&path)?;
        let path_str = path.display().to_string();
        workloads.push(Workload {
            name: path_str.clone(),
            quadlet_type: QuadletType::ConfigFile,
            quadlet_contents: contents,
            systemd_unit_name: path_str,
            enabled_state: EnabledState::Enabled,
            restart_policy: RestartPolicy::Always,
        });
    }
    Ok(())
}

fn systemctl_available() -> bool {
    let output = Command::new("systemctl")
        .arg("is-system-running")
        .output();

    match output {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("System has not been booted with systemd")
                || stderr.contains("Failed to connect to bus")
            {
                return false;
            }
            true
        }
        Err(_) => false,
    }
}

fn query_unit_state(unit: &str) -> Result<Option<ObservedUnit>, ObservedError> {
    let output = Command::new("systemctl")
        .arg("show")
        .arg(unit)
        .arg("--property=ActiveState,UnitFileState")
        .arg("--no-page")
        .output()
        .map_err(|err| ObservedError::SystemdQueryFailed(err.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found")
            || stderr.contains("not-found")
            || stderr.contains("Failed to connect to bus")
            || stderr.contains("System has not been booted with systemd")
        {
            return Ok(Some(ObservedUnit {
                unit_name: unit.to_string(),
                active_state: UnitActiveState::Inactive,
                enabled_state: EnabledState::Disabled,
            }));
        }
        return Err(ObservedError::SystemdQueryFailed(stderr.to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let active = parse_property(&stdout, "ActiveState").unwrap_or("inactive");
    let enabled = parse_property(&stdout, "UnitFileState").unwrap_or("disabled");

    Ok(Some(ObservedUnit {
        unit_name: unit.to_string(),
        active_state: map_active_state(active),
        enabled_state: map_enabled_state(enabled),
    }))
}

fn parse_property<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    for line in output.lines() {
        if let Some(value) = line.strip_prefix(&format!("{key}=")) {
            return Some(value.trim());
        }
    }
    None
}

fn map_active_state(value: &str) -> UnitActiveState {
    match value {
        "active" => UnitActiveState::Active,
        "failed" => UnitActiveState::Failed,
        _ => UnitActiveState::Inactive,
    }
}

fn map_enabled_state(value: &str) -> EnabledState {
    match value {
        "enabled" => EnabledState::Enabled,
        _ => EnabledState::Disabled,
    }
}
