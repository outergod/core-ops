use crate::core::audit::format_audit_record;
use crate::core::types::AuditRecord;

pub fn format_status(record: &AuditRecord) -> String {
    format!("last run\n{}", format_audit_record(record))
}
