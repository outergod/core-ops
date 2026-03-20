use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{PlanAction, PlanActionType, ReconciliationPlan, Workload};
use crate::core::unit::systemd_unit_for_quadlet_file;

#[derive(Debug)]
pub enum ApplyError {
    MissingQuadletDir(PathBuf),
    MissingWorkload(String),
    SystemdCommandFailed(String),
    Io(std::io::Error),
}

impl From<std::io::Error> for ApplyError {
    fn from(err: std::io::Error) -> Self {
        ApplyError::Io(err)
    }
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::MissingQuadletDir(path) => {
                write!(f, "missing quadlet dir: {}", path.display())
            }
            ApplyError::MissingWorkload(name) => write!(f, "missing workload: {}", name),
            ApplyError::SystemdCommandFailed(msg) => write!(f, "systemd command failed: {}", msg),
            ApplyError::Io(err) => write!(f, "apply io error: {}", err),
        }
    }
}

impl std::error::Error for ApplyError {}

pub struct ApplyOutcome {
    pub actions_applied: Vec<PlanAction>,
    pub files_written: Vec<String>,
    pub files_removed: Vec<String>,
}

pub fn apply_plan(
    plan: &ReconciliationPlan,
    desired_workloads: &[Workload],
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ApplyOutcome, ApplyError> {
    if !quadlet_dir.exists() {
        return Err(ApplyError::MissingQuadletDir(quadlet_dir.to_path_buf()));
    }

    let mut files_written = Vec::new();
    let mut files_removed = Vec::new();

    for action in &plan.actions {
        match &action.action_type {
            PlanActionType::WriteQuadlet => {
                let workload = find_workload(desired_workloads, &action.target)?;
                let path = quadlet_dir.join(&workload.systemd_unit_name);
                fs::write(&path, &workload.quadlet_contents)?;
                files_written.push(path.display().to_string());
            }
            PlanActionType::RemoveQuadlet => {
                let mut removed = false;
                for entry in fs::read_dir(quadlet_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        if file_name == action.target
                            || file_name.starts_with(&format!("{}.", action.target))
                        {
                            fs::remove_file(&path)?;
                            files_removed.push(path.display().to_string());
                            removed = true;
                        }
                    }
                }
                if !removed {
                    return Err(ApplyError::MissingWorkload(action.target.clone()));
                }
            }
            PlanActionType::EnableUnit => {
                return Err(ApplyError::SystemdCommandFailed(
                    "enable/disable is handled via Quadlet [Install], not systemctl enable"
                        .to_string(),
                ));
            }
            PlanActionType::DisableUnit => {
                return Err(ApplyError::SystemdCommandFailed(
                    "enable/disable is handled via Quadlet [Install], not systemctl disable"
                        .to_string(),
                ));
            }
            PlanActionType::StartUnit => {
                let unit = unit_name_for_start_stop(desired_workloads, quadlet_dir, &action.target)?;
                run_systemctl(&["start", &unit])?;
            }
            PlanActionType::StopUnit => {
                let unit = unit_name_for_start_stop(desired_workloads, quadlet_dir, &action.target)?;
                run_systemctl(&["stop", &unit])?;
            }
            PlanActionType::ReloadSystemd => {
                if reload_systemd {
                    run_systemctl(&["daemon-reload"])?;
                }
            }
            PlanActionType::Unknown(action) => {
                return Err(ApplyError::SystemdCommandFailed(format!(
                    "unsupported plan action: {}",
                    action
                )));
            }
        }
    }

    Ok(ApplyOutcome {
        actions_applied: plan.actions.clone(),
        files_written,
        files_removed,
    })
}

fn find_workload<'a>(
    workloads: &'a [Workload],
    name: &str,
) -> Result<&'a Workload, ApplyError> {
    workloads
        .iter()
        .find(|workload| workload.systemd_unit_name == name || workload.name == name)
        .ok_or_else(|| ApplyError::MissingWorkload(name.to_string()))
}

fn run_systemctl(args: &[&str]) -> Result<(), ApplyError> {
    let output = std::process::Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|err| ApplyError::SystemdCommandFailed(err.to_string()))?;
    if !output.status.success() {
        return Err(ApplyError::SystemdCommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(())
}


fn unit_name_for_start_stop(
    workloads: &[Workload],
    quadlet_dir: &Path,
    target: &str,
) -> Result<String, ApplyError> {
    if let Some(workload) = workloads
        .iter()
        .find(|w| w.systemd_unit_name == target || w.name == target)
    {
        return Ok(systemd_unit_for_quadlet_file(&workload.systemd_unit_name));
    }

    for entry in fs::read_dir(quadlet_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
            if file_name == target || file_name.starts_with(&format!("{target}.")) {
                return Ok(systemd_unit_for_quadlet_file(file_name));
            }
        }
    }

    Ok(systemd_unit_for_quadlet_file(target))
}
