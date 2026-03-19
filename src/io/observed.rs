use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::types::{EnabledState, ObservedState, ObservedUnit, UnitActiveState, Workload};
use crate::io::quadlet::{read_quadlet_dir, systemd_unit_for_quadlet_file, QuadletError};

#[derive(Debug)]
pub enum ObservedError {
    MissingQuadletDir(PathBuf),
    SystemdQueryFailed(String),
    Quadlet(QuadletError),
}

impl From<QuadletError> for ObservedError {
    fn from(err: QuadletError) -> Self {
        ObservedError::Quadlet(err)
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
        }
    }
}

impl std::error::Error for ObservedError {}

pub fn read_observed_state(
    quadlet_dir: &Path,
    observed_revision_id: Option<String>,
) -> Result<ObservedState, ObservedError> {
    if !quadlet_dir.exists() {
        return Err(ObservedError::MissingQuadletDir(quadlet_dir.to_path_buf()));
    }

    let workloads: Vec<Workload> = read_quadlet_dir(quadlet_dir)?;
    let units = read_systemd_units(&workloads)?;

    Ok(ObservedState {
        observed_revision_id,
        units,
        workloads,
        last_reconcile_id: None,
        host_info: None,
    })
}

fn read_systemd_units(workloads: &[Workload]) -> Result<Vec<ObservedUnit>, ObservedError> {
    if !systemctl_available() {
        log::warn!("systemctl unavailable; skipping unit discovery");
        return Ok(Vec::new());
    }

    let mut units = Vec::new();
    for workload in workloads {
        let unit_name = systemd_unit_for_quadlet_file(&workload.systemd_unit_name);
        match query_unit_state(&unit_name)? {
            Some(unit) => units.push(unit),
            None => {}
        }
    }

    Ok(units)
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
