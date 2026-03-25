use std::fs;
use std::path::Path;

use crate::core::audit::{format_audit_event_json, format_audit_record, AuditEvent};
use crate::core::types::AuditRecord;

#[derive(Debug)]
pub enum AuditError {
    Io(std::io::Error),
}

impl From<std::io::Error> for AuditError {
    fn from(err: std::io::Error) -> Self {
        AuditError::Io(err)
    }
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditError::Io(err) => write!(f, "audit io error: {}", err),
        }
    }
}

impl std::error::Error for AuditError {}

pub fn write_audit_record(dir: &Path, record: &AuditRecord) -> Result<String, AuditError> {
    fs::create_dir_all(dir)?;
    let file_name = format!("{}.log", record.record_id);
    let path = dir.join(&file_name);
    let body = format_audit_record(record);
    fs::write(&path, body)?;
    Ok(path.display().to_string())
}

pub fn emit_journal_event(event: &AuditEvent) -> Result<(), AuditError> {
    let payload = format_audit_event_json(event);
    let target = journal_target(event);
    log::info!(target: target, "{}", payload);
    Ok(())
}

pub fn journal_target(event: &AuditEvent) -> &'static str {
    if event.summary.contains("mount") {
        "audit.mount"
    } else if event.reconciliation_status.is_some() {
        "audit.provenance"
    } else {
        "audit"
    }
}
