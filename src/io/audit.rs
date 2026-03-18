use std::fs;
use std::path::Path;

use crate::core::audit::format_audit_record;
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
