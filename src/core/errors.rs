use crate::core::types::FailureClass;

#[derive(Clone, Debug, PartialEq, Eq)]
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationErrorKind {
    MissingInvariant,
    MissingBoundaryScope,
    DuplicateWorkload,
    DuplicateUnitName,
    UnsupportedQuadletType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
