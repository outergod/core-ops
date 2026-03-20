use std::path::PathBuf;

use crate::cli::apply as apply_cmd;
use crate::core::audit::build_audit_event;
use crate::core::errors::CoreError;
use crate::core::types::{FailureClass, ReconcileRun, RunLock};
use crate::io::audit as audit_io;
use crate::io::lock::FileRunLock;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub repo: String,
    pub rev: String,
    pub quadlet_dir: PathBuf,
    pub audit_dir: Option<PathBuf>,
    pub reload_systemd: bool,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct AgentOutput {
    pub run: ReconcileRun,
    pub report: String,
}

pub fn run_agent(config: &AgentConfig) -> Result<AgentOutput, CoreError> {
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
    );

    let release_result = lock
        .release(guard)
        .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()));

    let (result, report) = result?;
    let run = result.run;
    if let Err(err) = release_result {
        return Err(err);
    }

    let event = build_audit_event(&run, None);
    audit_io::emit_journal_event(&event)
        .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;

    if let Some(dir) = &config.audit_dir {
        let record = crate::core::audit::build_audit_record(
            &run.run_id,
            Vec::new(),
            &crate::core::types::ReconciliationPlan {
                plan_id: "agent".to_string(),
                desired_revision_id: config.rev.clone(),
                observed_revision_id: None,
                actions: Vec::new(),
                safety_checks: Vec::new(),
                expected_outcomes: Vec::new(),
            },
            result.verification_results,
        );
        let _ = audit_io::write_audit_record(dir, &record)
            .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))?;
    }

    Ok(AgentOutput { run, report })
}
