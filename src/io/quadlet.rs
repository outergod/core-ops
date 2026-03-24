use std::fs;
use std::path::{Path, PathBuf};

use crate::core::unit::{render_automount_unit, render_mount_unit};
use crate::core::types::MountDeclaration;
use crate::core::types::{EnabledState, QuadletType, RestartPolicy, Workload};

pub(crate) const SOCKET_MANAGED_MARKER: &str = "# managed-by: core-ops";

#[derive(Debug)]
pub enum QuadletError {
    UnsupportedExtension(String),
    Io(std::io::Error),
}

impl From<std::io::Error> for QuadletError {
    fn from(err: std::io::Error) -> Self {
        QuadletError::Io(err)
    }
}

impl std::fmt::Display for QuadletError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuadletError::UnsupportedExtension(ext) => {
                write!(f, "unsupported quadlet extension: {}", ext)
            }
            QuadletError::Io(err) => write!(f, "quadlet io error: {}", err),
        }
    }
}

impl std::error::Error for QuadletError {}

pub fn read_quadlet_dir(dir: &Path) -> Result<Vec<Workload>, QuadletError> {
    let mut workloads = Vec::new();
    read_quadlet_dir_inner(dir, &mut workloads)?;
    Ok(workloads)
}

fn read_quadlet_dir_inner(dir: &Path, workloads: &mut Vec<Workload>) -> Result<(), QuadletError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                if name.starts_with('.') {
                    continue;
                }
            }
            read_quadlet_dir_inner(&path, workloads)?;
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name,
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        match load_quadlet_file(&path) {
            Ok(workload) => workloads.push(workload),
            Err(QuadletError::UnsupportedExtension(ext)) => {
                log::warn!("unsupported quadlet extension: {}", ext);
            }
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

fn load_quadlet_file(path: &Path) -> Result<Workload, QuadletError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QuadletError::UnsupportedExtension("invalid filename".to_string()))?;

    let (name, quadlet_type) = parse_quadlet_name(file_name)?;

    let mut contents = fs::read_to_string(path)?;
    if quadlet_type == QuadletType::Socket {
        contents = normalize_socket_contents(&contents);
    }

    Ok(Workload {
        name,
        quadlet_type,
        quadlet_contents: contents,
        systemd_unit_name: file_name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    })
}

pub(crate) fn parse_quadlet_name(file_name: &str) -> Result<(String, QuadletType), QuadletError> {
    let path = PathBuf::from(file_name);
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QuadletError::UnsupportedExtension(file_name.to_string()))?;
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| QuadletError::UnsupportedExtension(file_name.to_string()))?;

    let quadlet_type = match ext {
        "container" => QuadletType::Container,
        "socket" => QuadletType::Socket,
        "pod" => QuadletType::Pod,
        "volume" => QuadletType::Volume,
        "network" => QuadletType::Network,
        _ => return Err(QuadletError::UnsupportedExtension(ext.to_string())),
    };

    Ok((stem.to_string(), quadlet_type))
}

pub(crate) fn normalize_socket_contents(contents: &str) -> String {
    if contents.contains(SOCKET_MANAGED_MARKER) {
        return contents.to_string();
    }
    format!("{SOCKET_MANAGED_MARKER}\n{contents}")
}

pub fn render_native_mount_units(declaration: &MountDeclaration) -> Vec<(String, String)> {
    let mut units = vec![render_mount_unit(declaration)];
    if let Some(automount) = render_automount_unit(declaration) {
        units.push(automount);
    }
    units
}
