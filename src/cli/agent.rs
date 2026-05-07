use std::path::PathBuf;

use crate::cli::apply as apply_cmd;
use crate::core::audit::build_audit_event;
use crate::core::errors::{CoreError, StateError};
use crate::core::types::{FailureClass, ReconcileRun, RunLock};
use crate::io::audit as audit_io;
use crate::io::lock::FileRunLock;
use crate::io::state::{read_persisted_state, resolve_state_file, STATE_FILE_ENV};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub quadlet_dir: PathBuf,
    pub audit_dir: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
    pub reload_systemd: bool,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum AgentExitReason {
    Completed(AgentOutput),
    Uninitialized,
    Detached { revision: String },
}

#[derive(Debug)]
pub struct AgentOutput {
    pub run: ReconcileRun,
    pub report: String,
}

pub fn run_agent(config: &AgentConfig) -> Result<AgentExitReason, CoreError> {
    let state_path = resolve_state_file(config.state_file.clone());

    let state = match read_persisted_state(&state_path) {
        Ok(Some(s)) => s,
        Ok(None) => return Ok(AgentExitReason::Uninitialized),
        Err(StateError::Corrupt(path)) => {
            return Err(CoreError::new(
                FailureClass::Apply,
                format!("state file at {path} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover"),
            ));
        }
        Err(err) => return Err(CoreError::new(FailureClass::Apply, err.to_string())),
    };

    if state.detached {
        let revision = state
            .reconciliation
            .last_applied_revision
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        return Ok(AgentExitReason::Detached { revision });
    }

    let repo = state.desired_state.repository.clone();
    let rev = state.desired_state.requested_ref.clone();

    std::env::set_var(STATE_FILE_ENV, &state_path);
    let lock_path = config
        .lock_path
        .clone()
        .unwrap_or_else(FileRunLock::default_path);
    let lock = FileRunLock::new(lock_path);
    let guard = lock
        .acquire()
        .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;

    let result = apply_cmd::apply_with_report(
        &apply_cmd::ApplyTarget::initd(&repo, &rev, Some(state_path.clone())),
        &config.quadlet_dir,
        config.reload_systemd,
    );

    let release_result = lock
        .release(guard)
        .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()));

    let output = result?;
    let run = output.result.run.clone();
    release_result?;

    let provenance = read_persisted_state(&state_path).ok().flatten();
    let event = build_audit_event(
        &run,
        Some(&output.plan),
        &output.result.verification_results,
        provenance.as_ref(),
    );
    audit_io::emit_journal_event(&event)
        .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;

    if let Some(dir) = &config.audit_dir {
        let record = crate::core::audit::build_audit_record(
            &run.run_id,
            Vec::new(),
            &output.plan,
            output.result.verification_results.clone(),
        );
        let _ = audit_io::write_audit_record(dir, &record)
            .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;
        if let Some(convergence) = output.result.convergence.as_ref() {
            let _ = audit_io::write_convergence_summary(dir, convergence)
                .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;
        }
    }

    Ok(AgentExitReason::Completed(AgentOutput {
        run,
        report: output.human_report,
    }))
}
