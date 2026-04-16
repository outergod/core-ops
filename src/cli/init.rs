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

    write_init_state(
        &state_path,
        &args.repository,
        &args.requested_ref,
        existing.as_ref(),
    )
    .map_err(|err| CoreError::new(FailureClass::Apply, err.to_string()))
}
