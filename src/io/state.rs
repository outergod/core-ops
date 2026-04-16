use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::NamedTempFile;

use crate::core::errors::StateError;
use crate::core::reconcile::{
    build_reconciliation_provenance, never_run_provenance, next_reconciliation_generation,
};
use crate::core::types::{
    ControllerProvenance, DesiredStateProvenance, DeterministicConvergenceRecord,
    DeterministicPersistedState, PersistedProvenanceState, ReconciliationProvenance,
    ReconciliationStatus, RetainedAppliedSnapshot, RollbackEligibility, RollbackTargetCandidate,
    TreeState,
};

pub const STATE_FILE_ENV: &str = "CORE_OPS_STATE_FILE";
pub const DEFAULT_STATE_FILE_PATH: &str = "/var/lib/core-ops/status.json";
pub const CONTROLLER_VERSION_ENV: &str = "CORE_OPS_CONTROLLER_VERSION";
pub const CONTROLLER_REVISION_ENV: &str = "CORE_OPS_CONTROLLER_REVISION";
pub const CONTROLLER_BUILD_TIME_ENV: &str = "CORE_OPS_CONTROLLER_BUILD_TIME";
pub const CONTROLLER_TREE_STATE_ENV: &str = "CORE_OPS_CONTROLLER_TREE_STATE";
// Operator-facing plan/apply/result views evolve independently from persisted on-disk state.
pub const DETERMINISTIC_STATE_FILE_NAME: &str = "deterministic-state.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationAttemptHandle {
    pub generation: u64,
    pub started_at: String,
}

pub fn read_persisted_state(path: &Path) -> Result<Option<PersistedProvenanceState>, StateError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(StateError::Io(err.to_string())),
    };

    match parse_persisted_state_text(&contents) {
        Some(state) => Ok(Some(state)),
        None => Err(StateError::Corrupt(path.display().to_string())),
    }
}

pub fn write_persisted_state(
    path: &Path,
    state: &PersistedProvenanceState,
) -> Result<(), StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::Io(format!("state path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|err| StateError::Io(err.to_string()))?;

    let body = serde_json::to_vec_pretty(state)
        .map_err(|err| StateError::Serialization(err.to_string()))?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|err| StateError::Io(err.to_string()))?;
    use std::io::Write;
    temp.write_all(&body)
        .and_then(|_| temp.flush())
        .map_err(|err| StateError::Io(err.to_string()))?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|err| StateError::Io(err.error.to_string()))
}

pub fn parse_persisted_state_text(contents: &str) -> Option<PersistedProvenanceState> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    let schema_version = value.get("schema_version")?.as_u64()? as u32;
    if !is_supported_schema_version(schema_version) {
        return None;
    }
    let state: PersistedProvenanceState = serde_json::from_value(value).ok()?;
    if !state.reconciliation.is_valid() {
        return None;
    }
    Some(state)
}

pub fn is_supported_schema_version(version: u32) -> bool {
    version == crate::core::types::PERSISTED_PROVENANCE_SCHEMA_VERSION
}

pub fn resolve_state_file(explicit: Option<PathBuf>) -> PathBuf {
    explicit
        .or_else(|| std::env::var_os(STATE_FILE_ENV).map(PathBuf::from))
        .unwrap_or_else(default_state_file_path)
}

pub fn default_state_file_path() -> PathBuf {
    PathBuf::from(DEFAULT_STATE_FILE_PATH)
}

pub fn persist_success_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
    observed_revision: &str,
) -> Result<(), StateError> {
    let attempt =
        persist_in_progress_state(path, repository, requested_ref, observed_revision, None)?;
    persist_finished_state(
        path,
        repository,
        requested_ref,
        observed_revision,
        None,
        &attempt,
        ReconciliationStatus::Success,
    )
}

/// Write controller state for a fresh `init` invocation.
///
/// If `existing` is `Some` and repository/ref are unchanged, reconciliation
/// history is preserved and only `detached` is cleared.  Otherwise a clean
/// NeverRun state is written.
pub fn write_init_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
    existing: Option<&PersistedProvenanceState>,
) -> Result<(), StateError> {
    let same_config = existing
        .map(|s| {
            s.desired_state.repository == repository
                && s.desired_state.requested_ref == requested_ref
        })
        .unwrap_or(false);

    if same_config {
        let mut state = existing.unwrap().clone();
        state.detached = false;
        state.controller = controller_provenance_from_env();
        write_persisted_state(path, &state)
    } else {
        persist_never_run_state(path, repository, requested_ref)
    }
}

pub fn persist_never_run_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
) -> Result<(), StateError> {
    let state = PersistedProvenanceState {
        schema_version: crate::core::types::PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: controller_provenance_from_env(),
        desired_state: DesiredStateProvenance {
            repository: repository.to_string(),
            requested_ref: requested_ref.to_string(),
            last_observed_revision: None,
            last_observed_at: None,
        },
        reconciliation: never_run_provenance(),
        detached: false,
    };

    write_persisted_state(path, &state)
}

pub fn persist_in_progress_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
    observed_revision: &str,
    attempted_revision: Option<&str>,
) -> Result<ReconciliationAttemptHandle, StateError> {
    let previous = read_persisted_state(path).unwrap_or(None);
    let generation = next_reconciliation_generation(previous.as_ref());
    let started_at = timestamp_string();
    let state = build_state(
        previous.as_ref(),
        repository,
        requested_ref,
        Some(observed_revision),
        Some(started_at.clone()),
        build_reconciliation_provenance(
            previous.as_ref(),
            generation,
            Some(observed_revision),
            attempted_revision,
            ReconciliationStatus::InProgress,
            Some(started_at.clone()),
            None,
        ),
    );
    write_persisted_state(path, &state)?;
    Ok(ReconciliationAttemptHandle {
        generation,
        started_at,
    })
}

pub fn persist_finished_state(
    path: &Path,
    repository: &str,
    requested_ref: &str,
    observed_revision: &str,
    attempted_revision: Option<&str>,
    attempt: &ReconciliationAttemptHandle,
    status: ReconciliationStatus,
) -> Result<(), StateError> {
    let previous = read_persisted_state(path).unwrap_or(None);
    let state = build_state(
        previous.as_ref(),
        repository,
        requested_ref,
        Some(observed_revision),
        Some(attempt.started_at.clone()),
        build_reconciliation_provenance(
            previous.as_ref(),
            attempt.generation,
            Some(observed_revision),
            attempted_revision,
            status,
            Some(attempt.started_at.clone()),
            Some(timestamp_string()),
        ),
    );
    write_persisted_state(path, &state)
}

fn build_state(
    previous: Option<&PersistedProvenanceState>,
    repository: &str,
    requested_ref: &str,
    observed_revision: Option<&str>,
    observed_at: Option<String>,
    reconciliation: ReconciliationProvenance,
) -> PersistedProvenanceState {
    PersistedProvenanceState {
        schema_version: crate::core::types::PERSISTED_PROVENANCE_SCHEMA_VERSION,
        controller: controller_provenance_from_env(),
        desired_state: DesiredStateProvenance {
            repository: repository.to_string(),
            requested_ref: requested_ref.to_string(),
            last_observed_revision: observed_revision.map(ToString::to_string).or_else(|| {
                previous.and_then(|state| {
                    state
                        .desired_state
                        .last_observed_revision
                        .as_ref()
                        .map(ToString::to_string)
                })
            }),
            last_observed_at: observed_at.or_else(|| {
                previous.and_then(|state| {
                    state
                        .desired_state
                        .last_observed_at
                        .as_ref()
                        .map(ToString::to_string)
                })
            }),
        },
        reconciliation,
        detached: false,
    }
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

pub fn default_deterministic_state_path() -> PathBuf {
    default_state_file_path()
        .parent()
        .unwrap_or_else(|| Path::new("/var/lib/core-ops"))
        .join(DETERMINISTIC_STATE_FILE_NAME)
}

pub fn read_deterministic_state(
    path: &Path,
) -> Result<Option<DeterministicPersistedState>, StateError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(StateError::Io(err.to_string())),
    };
    let state = serde_json::from_str(&contents)
        .map_err(|err| StateError::Serialization(err.to_string()))?;
    Ok(Some(state))
}

pub fn write_deterministic_state(
    path: &Path,
    state: &DeterministicPersistedState,
) -> Result<(), StateError> {
    let parent = path
        .parent()
        .ok_or_else(|| StateError::Io(format!("state path has no parent: {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|err| StateError::Io(err.to_string()))?;
    let body = serde_json::to_vec_pretty(state)
        .map_err(|err| StateError::Serialization(err.to_string()))?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|err| StateError::Io(err.to_string()))?;
    use std::io::Write;
    temp.write_all(&body)
        .and_then(|_| temp.flush())
        .map_err(|err| StateError::Io(err.to_string()))?;
    temp.persist(path)
        .map(|_| ())
        .map_err(|err| StateError::Io(err.error.to_string()))
}

pub fn resolve_rollback_target(
    state: &DeterministicPersistedState,
    scope_id: &str,
    target_revision_id: &str,
) -> RollbackTargetCandidate {
    match state
        .retained_snapshots
        .iter()
        .find(|snapshot| snapshot.revision_id == target_revision_id)
    {
        None => RollbackTargetCandidate {
            target_revision_id: target_revision_id.to_string(),
            scope_id: scope_id.to_string(),
            eligibility: RollbackEligibility::MissingSnapshot,
            reason: "retained successful snapshot not found".to_string(),
        },
        Some(snapshot) if snapshot.scope_id != scope_id => RollbackTargetCandidate {
            target_revision_id: target_revision_id.to_string(),
            scope_id: scope_id.to_string(),
            eligibility: RollbackEligibility::IncompatibleScope,
            reason: format!(
                "snapshot for revision {} was recorded on scope {}, which is incompatible with current scope {}",
                target_revision_id, snapshot.scope_id, scope_id
            ),
        },
        Some(snapshot) if !snapshot.retained => RollbackTargetCandidate {
            target_revision_id: target_revision_id.to_string(),
            scope_id: scope_id.to_string(),
            eligibility: RollbackEligibility::Expired,
            reason: "retained snapshot expired from rollback window".to_string(),
        },
        Some(_) => RollbackTargetCandidate {
            target_revision_id: target_revision_id.to_string(),
            scope_id: scope_id.to_string(),
            eligibility: RollbackEligibility::Eligible,
            reason: "retained successful snapshot is rollback-eligible".to_string(),
        },
    }
}

pub fn retained_snapshot_for_target<'a>(
    state: &'a DeterministicPersistedState,
    scope_id: &str,
    target_revision_id: &str,
) -> Option<&'a RetainedAppliedSnapshot> {
    state.retained_snapshots.iter().find(|snapshot| {
        snapshot.revision_id == target_revision_id
            && snapshot.scope_id == scope_id
            && snapshot.retained
    })
}

pub fn latest_retained_snapshot_for_scope<'a>(
    state: &'a DeterministicPersistedState,
    scope_id: &str,
) -> Option<&'a RetainedAppliedSnapshot> {
    state
        .retained_snapshots
        .iter()
        .rev()
        .find(|snapshot| snapshot.scope_id == scope_id && snapshot.retained)
}

pub fn retain_successful_snapshot(
    state: &mut DeterministicPersistedState,
    snapshot: RetainedAppliedSnapshot,
    max_retained: usize,
) {
    state.retained_snapshots.retain(|existing| {
        !(existing.scope_id == snapshot.scope_id && existing.revision_id == snapshot.revision_id)
    });
    state.retained_snapshots.push(snapshot);

    let retained_count = state
        .retained_snapshots
        .iter()
        .filter(|entry| entry.retained)
        .count();
    if retained_count <= max_retained {
        return;
    }

    let mut overflow = retained_count - max_retained;
    for entry in &mut state.retained_snapshots {
        if overflow == 0 {
            break;
        }
        if entry.retained {
            entry.retained = false;
            overflow -= 1;
        }
    }
}

pub fn record_rollback_outcome(
    state: &mut DeterministicPersistedState,
    target: RollbackTargetCandidate,
    convergence: DeterministicConvergenceRecord,
) {
    state.current_scope = convergence.scope_id.clone();
    state.latest_rollback_target = Some(target);
    state.latest_convergence = Some(convergence);
}

pub fn record_convergence_outcome(
    state: &mut DeterministicPersistedState,
    convergence: DeterministicConvergenceRecord,
) {
    state.current_scope = convergence.scope_id.clone();
    state.latest_convergence = Some(convergence);
}
