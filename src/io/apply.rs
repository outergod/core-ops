use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{PlanAction, PlanActionType, ReconciliationPlan, Workload};

#[derive(Debug)]
pub enum ApplyError {
    MissingQuadletDir(PathBuf),
    MissingWorkload(String),
    SystemdReloadFailed(String),
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
            ApplyError::SystemdReloadFailed(msg) => write!(f, "systemd reload failed: {}", msg),
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
    let mut needs_reload = false;

    for action in &plan.actions {
        match action.action_type {
            PlanActionType::WriteQuadlet => {
                let workload = find_workload(desired_workloads, &action.target)?;
                let path = quadlet_dir.join(&workload.systemd_unit_name);
                fs::write(&path, &workload.quadlet_contents)?;
                files_written.push(path.display().to_string());
                needs_reload = true;
            }
            PlanActionType::RemoveQuadlet => {
                for entry in fs::read_dir(quadlet_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        if file_name.starts_with(&format!("{}.", action.target)) {
                            fs::remove_file(&path)?;
                            files_removed.push(path.display().to_string());
                            needs_reload = true;
                        }
                    }
                }
            }
            _ => {
                // No-op for unit enable/disable/start/stop/reload in the MVP adapter.
            }
        }
    }

    if reload_systemd && needs_reload {
        let output = std::process::Command::new("systemctl")
            .arg("daemon-reload")
            .output()
            .map_err(|err| ApplyError::SystemdReloadFailed(err.to_string()))?;
        if !output.status.success() {
            return Err(ApplyError::SystemdReloadFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
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
        .find(|workload| workload.name == name)
        .ok_or_else(|| ApplyError::MissingWorkload(name.to_string()))
}
