use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::{EnabledState, QuadletType, RestartPolicy, Workload};

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

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let workload = load_quadlet_file(&path)?;
        workloads.push(workload);
    }

    Ok(workloads)
}

fn load_quadlet_file(path: &Path) -> Result<Workload, QuadletError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| QuadletError::UnsupportedExtension("invalid filename".to_string()))?;

    let (name, quadlet_type) = parse_quadlet_name(file_name)?;

    let contents = fs::read_to_string(path)?;

    Ok(Workload {
        name,
        quadlet_type,
        quadlet_contents: contents,
        systemd_unit_name: file_name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    })
}

fn parse_quadlet_name(file_name: &str) -> Result<(String, QuadletType), QuadletError> {
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
        "pod" => QuadletType::Pod,
        "volume" => QuadletType::Volume,
        "network" => QuadletType::Network,
        _ => return Err(QuadletError::UnsupportedExtension(ext.to_string())),
    };

    Ok((stem.to_string(), quadlet_type))
}
