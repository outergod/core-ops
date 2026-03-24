use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{
    DesiredState, MountDeclaration, PlanAction, PlanActionType, QuadletType, ReconciliationPlan,
    Workload,
};
use crate::core::unit::systemd_unit_for_quadlet_file;
use crate::io::systemd::systemd_unit_dir;

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
    let desired = DesiredState {
        repository_ref: String::new(),
        revision_id: String::new(),
        workloads: desired_workloads.to_vec(),
        mount_declarations: Vec::new(),
        mount_dependencies: Vec::new(),
        managed_config_paths: Vec::new(),
        managed_config_roots: Vec::new(),
        invariants: Vec::new(),
        boundaries: crate::core::types::Boundaries { scopes: Vec::new() },
    };
    apply_plan_with_desired(plan, &desired, quadlet_dir, reload_systemd)
}

pub fn apply_plan_with_desired(
    plan: &ReconciliationPlan,
    desired: &DesiredState,
    quadlet_dir: &Path,
    reload_systemd: bool,
) -> Result<ApplyOutcome, ApplyError> {
    if !quadlet_dir.exists() {
        return Err(ApplyError::MissingQuadletDir(quadlet_dir.to_path_buf()));
    }

    let mut files_written = Vec::new();
    let mut files_removed = Vec::new();

    prepare_target_paths(&desired.mount_declarations)?;

    let mut deferred_units: Vec<(PlanActionType, String)> = Vec::new();

    for action in &plan.actions {
        match &action.action_type {
            PlanActionType::PreparePath => {
                fs::create_dir_all(&action.target)?;
                files_written.push(action.target.clone());
            }
            PlanActionType::WriteQuadlet => {
                let workload = find_workload(&desired.workloads, &action.target)?;
                let path = if workload.quadlet_type == QuadletType::ConfigFile {
                    PathBuf::from(&workload.systemd_unit_name)
                } else {
                    target_dir_for_workload(quadlet_dir, workload).join(&workload.systemd_unit_name)
                };
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, &workload.quadlet_contents)?;
                files_written.push(path.display().to_string());
            }
            PlanActionType::RemoveQuadlet => {
                if action.target.contains(".socket.d/") {
                    let path = systemd_unit_dir().join(&action.target);
                    if path.exists() {
                        fs::remove_file(&path)?;
                        files_removed.push(path.display().to_string());
                        if let Some(parent) = path.parent() {
                            if parent.read_dir()?.next().is_none() {
                                let _ = fs::remove_dir(parent);
                            }
                        }
                    } else {
                        return Err(ApplyError::MissingWorkload(action.target.clone()));
                    }
                } else if action.target.starts_with("/etc/") {
                    let path = PathBuf::from(&action.target);
                    if path.exists() {
                        fs::remove_file(&path)?;
                        files_removed.push(path.display().to_string());
                    } else {
                        return Err(ApplyError::MissingWorkload(action.target.clone()));
                    }
                } else {
                    let target_dir = target_dir_for_name(quadlet_dir, &action.target);
                    let mut removed = false;
                    for entry in fs::read_dir(&target_dir)? {
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
            }
            PlanActionType::EnableUnit => {
                // Quadlet-generated units rely on [Install] processing; no enable call is needed.
            }
            PlanActionType::DisableUnit => {
                // Quadlet-generated units rely on [Install] processing; no disable call is needed.
            }
            PlanActionType::StartUnit => {
                deferred_units.push((PlanActionType::StartUnit, action.target.clone()));
            }
            PlanActionType::RestartUnit => {
                deferred_units.push((PlanActionType::RestartUnit, action.target.clone()));
            }
            PlanActionType::StopUnit => {
                let unit =
                    unit_name_for_start_stop(&desired.workloads, quadlet_dir, &action.target)?;
                run_systemctl_allow_not_loaded(&["stop", &unit])?;
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

    let mut restarted = std::collections::HashSet::new();
    let mut started = std::collections::HashSet::new();
    for (action_type, target) in deferred_units {
        match action_type {
            PlanActionType::RestartUnit => {
                let unit = unit_name_for_start_stop(&desired.workloads, quadlet_dir, &target)?;
                run_systemctl(&["restart", &unit])?;
                restarted.insert(target.clone());
                started.insert(target);
            }
            PlanActionType::StartUnit => {
                if restarted.contains(&target) {
                    continue;
                }
                if started.contains(&target) {
                    continue;
                }
                let unit = unit_name_for_start_stop(&desired.workloads, quadlet_dir, &target)?;
                run_systemctl(&["start", &unit])?;
                started.insert(target);
            }
            _ => {}
        }
    }

    Ok(ApplyOutcome {
        actions_applied: plan.actions.clone(),
        files_written,
        files_removed,
    })
}

fn target_dir_for_workload(quadlet_dir: &Path, workload: &Workload) -> PathBuf {
    match workload.quadlet_type {
        QuadletType::Socket | QuadletType::SocketDropIn | QuadletType::Mount | QuadletType::Automount => systemd_unit_dir(),
        QuadletType::ConfigFile => PathBuf::from("/"),
        _ => quadlet_dir.to_path_buf(),
    }
}

fn target_dir_for_name(quadlet_dir: &Path, target: &str) -> PathBuf {
    if target.contains(".socket.d/")
        || Path::new(target)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| matches!(ext, "socket" | "mount" | "automount"))
            == Some(true)
    {
        systemd_unit_dir()
    } else if target.starts_with("/etc/") {
        PathBuf::from("/")
    } else {
        quadlet_dir.to_path_buf()
    }
}

fn prepare_target_paths(mounts: &[MountDeclaration]) -> Result<(), ApplyError> {
    for mount in mounts {
        let Some(prepared) = &mount.prepared_path else {
            continue;
        };
        if prepared.create_if_missing {
            fs::create_dir_all(&prepared.path)?;
        }
        if let Some(mode) = &prepared.mode {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let parsed = u32::from_str_radix(mode, 8).map_err(|_| {
                    ApplyError::SystemdCommandFailed(format!("invalid mode: {}", mode))
                })?;
                let perms = fs::Permissions::from_mode(parsed);
                fs::set_permissions(&prepared.path, perms)?;
            }
        }
        #[cfg(unix)]
        if prepared.owner.is_some() || prepared.group.is_some() {
            let uid = prepared
                .owner
                .as_deref()
                .map(str::parse::<u32>)
                .transpose()
                .map_err(|_| ApplyError::SystemdCommandFailed("invalid owner uid".to_string()))?
                .unwrap_or(u32::MAX);
            let gid = prepared
                .group
                .as_deref()
                .map(str::parse::<u32>)
                .transpose()
                .map_err(|_| ApplyError::SystemdCommandFailed("invalid group gid".to_string()))?
                .unwrap_or(u32::MAX);
            let path = std::ffi::CString::new(prepared.path.clone())
                .map_err(|_| ApplyError::SystemdCommandFailed("invalid prepared path".to_string()))?;
            let result = unsafe { libc::chown(path.as_ptr(), uid, gid) };
            if result != 0 {
                return Err(ApplyError::Io(std::io::Error::last_os_error()));
            }
        }
    }
    Ok(())
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

fn run_systemctl_allow_not_loaded(args: &[&str]) -> Result<(), ApplyError> {
    let output = std::process::Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|err| ApplyError::SystemdCommandFailed(err.to_string()))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not loaded") || stderr.contains("not found") {
        return Ok(());
    }
    Err(ApplyError::SystemdCommandFailed(stderr.to_string()))
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
