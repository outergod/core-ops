#![allow(unused_assignments)]

use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use miette::{Diagnostic, Report};
use thiserror::Error;

#[allow(unused_assignments)]
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
pub struct CliDiagnostic {
    message: String,
    #[help]
    help: Option<String>,
}

impl CliDiagnostic {
    fn from_core(err: &CoreError, attempts: Option<usize>, retryable: bool) -> Self {
        let class = class_label(&err.class);
        let message = format!("{} error: {}", class, err.message);
        let help = match mount_help(&err.message) {
            Some(help) => Some(help),
            None => attempts.map(|count| format!("attempts: {}, retryable: {}", count, retryable)),
        };
        Self { message, help }
    }
}

pub fn report_from_error(err: CoreError) -> Report {
    Report::new(CliDiagnostic::from_core(&err, None, err.is_retryable()))
}

pub fn report_from_retry(err: CoreError, attempts: usize, retryable: bool) -> Report {
    Report::new(CliDiagnostic::from_core(&err, Some(attempts), retryable))
}

fn class_label(class: &FailureClass) -> &'static str {
    match class {
        FailureClass::Validation => "validation",
        FailureClass::Plan => "plan",
        FailureClass::Apply => "apply",
        FailureClass::Verify => "verify",
        FailureClass::Transient => "transient",
    }
}

fn mount_help(message: &str) -> Option<String> {
    if message.contains("busy mount removal") {
        return Some("stop dependent managed services and ensure the mount target is no longer active before retrying".to_string());
    }
    if message.contains("degraded:") {
        return Some("the mount unit is present but the target path is no longer mounted; restore the backing mount and retry".to_string());
    }
    if message.contains("blocked:") || message.contains("mount target not mounted") {
        return Some("the required mount is not active yet; fix the mount source or unit state before retrying".to_string());
    }
    None
}
