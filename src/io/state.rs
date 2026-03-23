use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::NamedTempFile;

use crate::core::errors::StateError;
use crate::core::types::{
    ControllerProvenance, DesiredStateProvenance, PersistedProvenanceState,
    ReconciliationProvenance, ReconciliationStatus, TreeState,
};

pub const STATE_FILE_ENV: &str = "CORE_OPS_STATE_FILE";
pub const CONTROLLER_VERSION_ENV: &str = "CORE_OPS_CONTROLLER_VERSION";
pub const CONTROLLER_REVISION_ENV: &str = "CORE_OPS_CONTROLLER_REVISION";
pub const CONTROLLER_BUILD_TIME_ENV: &str = "CORE_OPS_CONTROLLER_BUILD_TIME";
pub const CONTROLLER_TREE_STATE_ENV: &str = "CORE_OPS_CONTROLLER_TREE_STATE";

pub fn read_persisted_state(path: &Path) -> Result<Option<PersistedProvenanceState>, StateError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(StateError::Io(err.to_string())),
    };

    let state: PersistedProvenanceState = match serde_json::from_str(&contents) {
        Ok(state) => state,
        Err(_) => return Ok(None),
    };

    if !state.is_supported_schema() || !state.reconciliation.is_valid() {
        return Ok(None);
    }

    Ok(Some(state))
}

pub fn write_persisted_state(
    path: &Path,
    state: &PersistedProvenanceState,
) -> Result<(), StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::Io(format!("state path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|err| StateError::Io(err.to_string()))?;

    let body =
        serde_json::to_vec_pretty(state).map_err(|err| StateError::Serialization(err.to_string()))?;
    let mut temp =
        NamedTempFile::new_in(parent).map_err(|err| StateError::Io(err.to_string()))?;
    use std::io::Write;
    temp.write_all(&body)
        .and_then(|_| temp.flush())
        .map_err(|err| StateError::Io(err.to_string()))?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|err| StateError::Io(err.error.to_string()))
}

pub fn resolve_state_file(explicit: Option<PathBuf>) -> Option<PathBuf> {
    explicit.or_else(|| std::env::var_os(STATE_FILE_ENV).map(PathBuf::from))
}

pub fn persist_success_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
    observed_revision: &str,
) -> Result<(), StateError> {
    let next_generation = read_persisted_state(path)?
        .map(|state| state.reconciliation.generation + 1)
        .unwrap_or(1);
    let now = timestamp_string();

    let state = PersistedProvenanceState {
        schema_version: crate::core::types::PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: controller_provenance_from_env(),
        desired_state: DesiredStateProvenance {
            repository: repository.to_string(),
            requested_ref: requested_ref.to_string(),
            last_observed_revision: Some(observed_revision.to_string()),
            last_observed_at: Some(now.clone()),
        },
        reconciliation: ReconciliationProvenance {
            generation: next_generation,
            status: ReconciliationStatus::Success,
            running: false,
            last_attempted_revision: Some(observed_revision.to_string()),
            last_applied_revision: Some(observed_revision.to_string()),
            last_started_at: Some(now.clone()),
            last_finished_at: Some(now),
            attempted_observed_divergence: None,
        },
    };

    write_persisted_state(path, &state)
}

fn controller_provenance_from_env() -> ControllerProvenance {
    let tree_state = match std::env::var(CONTROLLER_TREE_STATE_ENV).ok().as_deref() {
        Some("clean") => TreeState::Clean,
        Some("dirty") => TreeState::Dirty,
        _ => TreeState::Unknown,
    };

    ControllerProvenance {
        version: std::env::var(CONTROLLER_VERSION_ENV).ok(),
        revision: std::env::var(CONTROLLER_REVISION_ENV).ok(),
        build_time: std::env::var(CONTROLLER_BUILD_TIME_ENV).ok(),
        tree_state,
    }
}

fn timestamp_string() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}
