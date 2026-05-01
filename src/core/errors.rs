use std::path::PathBuf;

use crate::core::types::FailureClass;
use miette::{Diagnostic, NamedSource, SourceSpan};
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
    DuplicateMountId,
    DuplicateMountTarget,
    MissingMountReference,
    InvalidMountTarget,
    InvalidPreparedPath,
    InvalidPreparedOwnership,
    InvalidMountOwnershipScope,
    InvalidAutomount,
    ConflictingMountDefinition,
    InvalidObjectIdentity,
    SemanticDependencyCycle,
    RollbackIneligible,
    InvalidRetrySignature,
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
    #[error("state file is corrupt: {0}")]
    Corrupt(String),
}

/// Error class returned by the source-repository loader (`src/io/repo.rs`).
///
/// Unlike the rest of this module, `LayoutError` derives `miette::Diagnostic`
/// in addition to `thiserror::Error`. This is a deliberate asymmetry: the
/// parser benefits from source-span pointers into the offending YAML key or
/// `host.yaml` services list entry. Other errors in this module remain
/// plain `thiserror`.
#[derive(Debug, Error, Diagnostic)]
pub enum LayoutError {
    #[error("legacy layout artifact: {path}")]
    #[diagnostic(help(
        "see specs/016-source-repository-layout/contracts/layout.md and run \
         scripts/migrate-legacy-source-repo.sh"
    ))]
    LegacyArtifact { path: PathBuf },

    #[error("reserved name '{name}' (must not begin with '_' or '.')")]
    ReservedName { name: String },

    #[error("service '{service}' selected by host '{host}' has no directory under services/")]
    MissingService {
        host: String,
        service: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("declared here")]
        span: SourceSpan,
    },

    #[error("malformed service.yaml: unknown key '{key}'")]
    UnknownServiceManifestKey {
        key: String,
        #[source_code]
        src: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },

    #[error("malformed service.yaml: {message}")]
    ServiceManifestParse {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },

    #[error("malformed host.yaml: {message}")]
    HostManifestParse {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label]
        span: SourceSpan,
    },

    #[error("config file destination escapes /etc/{config_root}/: {source_path}")]
    ConfigEscape {
        config_root: String,
        source_path: PathBuf,
    },

    #[error("destination conflict at {target}: {a} and {b}")]
    DestinationConflict {
        target: PathBuf,
        a: PathBuf,
        b: PathBuf,
    },

    #[error("host overlay introduces base unit at {path} (only drop-ins and config replacements allowed)")]
    HostOverlayBaseUnit { path: PathBuf },

    #[error("orphan drop-in at {path} (no matching unit '{unit}' in merged set)")]
    OrphanDropIn { path: PathBuf, unit: String },
}
