use crate::core::types::FailureClass;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct CoreError {
    pub class: FailureClass,
    pub message: String,
}

impl CoreError {
    pub fn new(class: FailureClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.class == FailureClass::Transient
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationErrorKind {
    MissingInvariant,
    MissingBoundaryScope,
    DuplicateWorkload,
    DuplicateUnitName,
    UnsupportedQuadletType,
    UndefinedServiceSelection,
    MissingArtifactTarget,
    InvalidDropInOrdering,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct ValidationError {
    pub kind: ValidationErrorKind,
    pub message: String,
}

impl ValidationError {
    pub fn new(kind: ValidationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct EvaluationError {
    pub message: String,
}

impl EvaluationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RunLockError {
    #[error("run lock already held")]
    AlreadyHeld,
    #[error("run lock io error: {0}")]
    Io(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StateError {
    #[error("state io error: {0}")]
    Io(String),
    #[error("state serialization error: {0}")]
    Serialization(String),
}
