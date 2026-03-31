use std::path::PathBuf;

use crate::cli::apply as apply_cmd;
use crate::core::audit::build_audit_event;
use crate::core::errors::CoreError;
use crate::core::types::{FailureClass, ReconcileRun, RunLock};
use crate::io::audit as audit_io;
use crate::io::lock::FileRunLock;
use crate::io::state::{
    persist_never_run_state, read_persisted_state, resolve_state_file, STATE_FILE_ENV,
};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub repo: String,
    pub rev: String,
    pub quadlet_dir: PathBuf,
    pub audit_dir: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
    pub reload_systemd: bool,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct AgentOutput {
    pub run: ReconcileRun,
    pub report: String,
}

pub fn run_agent(config: &AgentConfig) -> Result<AgentOutput, CoreError> {
    let state_path = resolve_state_file(config.state_file.clone());
    if !state_path.exists() {
        persist_never_run_state(&state_path, &config.repo, &config.rev)
            .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;
    }
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
        &config.repo,
        &config.rev,
        &config.quadlet_dir,
        config.reload_systemd,
        Some(state_path.clone()),
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

    Ok(AgentOutput {
        run,
        report: output.human_report,
    })
}
