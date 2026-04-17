use std::process::Command;

use crate::cli::args::InitArgs;
use crate::core::errors::{CoreError, StateError};
use crate::core::types::FailureClass;
use crate::io::state::{read_persisted_state, resolve_state_file, write_init_state};

pub fn run_init(args: &InitArgs) -> Result<(), CoreError> {
    let state_path = resolve_state_file(args.state_file.clone());

    let existing = match read_persisted_state(&state_path) {
        Ok(state) => state,
        Err(StateError::Corrupt(path)) => {
            if !args.force {
                return Err(CoreError::new(
                    FailureClass::Validation,
                    format!(
                        "state file at {} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover",
                        path
                    ),
                ));
            }
            None
        }
        Err(err) => {
            return Err(CoreError::new(FailureClass::Apply, err.to_string()));
        }
    };

    if existing.is_some() && !args.force {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "already initialized ({}); use --force to reinitialize",
                state_path.display()
            ),
        ));
    }

    validate_repository_and_ref(&args.repository, &args.requested_ref)?;

    write_init_state(
        &state_path,
        &args.repository,
        &args.requested_ref,
        existing.as_ref(),
    )
    .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))
}

fn validate_repository_and_ref(repository: &str, requested_ref: &str) -> Result<(), CoreError> {
    let output = Command::new("git")
        .arg("ls-remote")
        .arg("--exit-code")
        .arg(repository)
        .arg(requested_ref)
        .output()
        .map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to invoke git to validate repository: {err}"),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "repository '{}' is not reachable or ref '{}' was not found; \
                 use a branch or tag name (not a commit SHA): {}",
                repository,
                requested_ref,
                stderr.trim()
            ),
        ));
    }

    if output.stdout.is_empty() {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "ref '{}' was not found in repository '{}'; \
                 use a branch or tag name, not a commit SHA",
                requested_ref, repository
            ),
        ));
    }

    Ok(())
}
