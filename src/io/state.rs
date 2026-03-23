use std::fs;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use crate::core::errors::StateError;
use crate::core::types::PersistedProvenanceState;

pub const STATE_FILE_ENV: &str = "CORE_OPS_STATE_FILE";

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
