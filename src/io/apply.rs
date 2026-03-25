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

    validate_runtime_unit_targets(plan, &desired.workloads, quadlet_dir)?;

    let mut files_written = Vec::new();
    let mut files_removed = Vec::new();

    prepare_target_paths(&desired.mount_declarations)?;

    let mut deferred_units: Vec<(PlanActionType, String)> = Vec::new();

    for action in &plan.actions {
        match &action.action_type {
            PlanActionType::PreparePath => {
                if should_skip_prepare_path(&desired.mount_declarations, &action.target) {
                    files_written.push(action.target.clone());
                    continue;
                }
                ensure_mountpoint_path(&action.target)?;
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
                if is_mount_unit_target(&action.target) {
                    stop_managed_service_workloads(&desired.workloads, quadlet_dir)?;
                    if let Some(target_path) = target_path_for_mount_unit(&action.target)? {
                        if is_mount_target_active(&target_path) {
                            return Err(ApplyError::SystemdCommandFailed(format!(
                                "busy mount removal: {}",
                                target_path
                            )));
                        }
                    }
                }
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

fn validate_runtime_unit_targets(
    plan: &ReconciliationPlan,
    workloads: &[Workload],
    quadlet_dir: &Path,
) -> Result<(), ApplyError> {
    for action in &plan.actions {
        if matches!(
            action.action_type,
            PlanActionType::StartUnit | PlanActionType::RestartUnit | PlanActionType::StopUnit
        ) {
            let _ = unit_name_for_start_stop(workloads, quadlet_dir, &action.target)?;
        }
    }
    Ok(())
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
            if should_skip_prepare_for_mount(mount) {
                continue;
            }
            ensure_mountpoint_path(&prepared.path)?;
        } else if !Path::new(&prepared.path).exists() {
            return Err(ApplyError::SystemdCommandFailed(format!(
                "mountpoint missing and CreateMountpoint=false: {}",
                prepared.path
            )));
        }
    }
    Ok(())
}

fn should_skip_prepare_path(mounts: &[MountDeclaration], target_path: &str) -> bool {
    mounts.iter().any(|mount| {
        mount.target_path == target_path
            || mount
                .prepared_path
                .as_ref()
                .map(|prepared| prepared.path == target_path)
                .unwrap_or(false)
    }) && mounts.iter().any(|mount| {
        (mount.target_path == target_path
            || mount
                .prepared_path
                .as_ref()
                .map(|prepared| prepared.path == target_path)
                .unwrap_or(false))
            && should_skip_prepare_for_mount(mount)
    })
}

fn should_skip_prepare_for_mount(mount: &MountDeclaration) -> bool {
    let unit_dir = systemd_unit_dir();
    unit_dir.join(mount.mount_unit_name()).exists()
        || mount
            .automount_unit_name()
            .map(|unit_name| unit_dir.join(unit_name).exists())
            .unwrap_or(false)
}

fn ensure_mountpoint_path(path: &str) -> Result<(), ApplyError> {
    let path = Path::new(path);
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_dir() {
                return Ok(());
            }
            if metadata.file_type().is_symlink() && path.is_dir() {
                return Ok(());
            }
            return Err(ApplyError::SystemdCommandFailed(format!(
                "mountpoint exists and is not a directory: {}",
                path.display()
            )));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(ApplyError::Io(err)),
    }

    match fs::create_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err)
            if err.kind() == std::io::ErrorKind::AlreadyExists
                && fs::symlink_metadata(path)
                    .map(|metadata| {
                        metadata.file_type().is_dir()
                            || (metadata.file_type().is_symlink() && path.is_dir())
                    })
                    .unwrap_or(false) =>
        {
            Ok(())
        }
        Err(err) => Err(ApplyError::Io(err)),
    }
}

fn is_mount_unit_target(target: &str) -> bool {
    target.ends_with(".mount") || target.ends_with(".automount")
}

fn stop_managed_service_workloads(workloads: &[Workload], quadlet_dir: &Path) -> Result<(), ApplyError> {
    for workload in workloads {
        if matches!(workload.quadlet_type, QuadletType::Container | QuadletType::Pod) {
            let unit = unit_name_for_start_stop(workloads, quadlet_dir, &workload.systemd_unit_name)?;
            run_systemctl_allow_not_loaded(&["stop", &unit])?;
        }
    }
    Ok(())
}

fn target_path_for_mount_unit(unit_name: &str) -> Result<Option<String>, ApplyError> {
    let path = systemd_unit_dir().join(unit_name);
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .find_map(|line| line.strip_prefix("Where=").map(str::to_string)))
}

fn is_mount_target_active(target_path: &str) -> bool {
    let mountinfo_path = std::env::var("CORE_OPS_MOUNTINFO_PATH")
        .unwrap_or_else(|_| "/proc/self/mountinfo".to_string());
    let Ok(contents) = fs::read_to_string(mountinfo_path) else {
        return false;
    };
    contents.lines().any(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        fields.get(4).copied() == Some(target_path)
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

    if Path::new(target)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext, "service" | "socket" | "mount" | "automount"))
        == Some(true)
    {
        return Ok(target.to_string());
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

    Err(ApplyError::MissingWorkload(target.to_string()))
}
