use crate::core::errors::CoreError;
use crate::core::types::FailureClass;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliErrorReport {
    pub class: FailureClass,
    pub message: String,
    pub attempts: Option<usize>,
    pub retryable: bool,
}

impl CliErrorReport {
    pub fn render(&self) -> String {
        let mut output = format!("failure[{}]: {}", class_label(&self.class), self.message);
        if let Some(attempts) = self.attempts {
            output.push_str(&format!(" (attempts: {}, retryable: {})", attempts, self.retryable));
        }
        output
    }
}

pub fn report_from_error(err: &CoreError) -> CliErrorReport {
    CliErrorReport {
        class: err.class.clone(),
        message: err.message.clone(),
        attempts: None,
        retryable: err.is_retryable(),
    }
}

pub fn report_from_retry(err: &CoreError, attempts: usize, retryable: bool) -> CliErrorReport {
    CliErrorReport {
        class: err.class.clone(),
        message: err.message.clone(),
        attempts: Some(attempts),
        retryable,
    }
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
