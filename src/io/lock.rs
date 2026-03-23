use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::core::errors::RunLockError;
use crate::core::types::{RunLock, RunLockGuard};

pub struct FileRunLock {
    path: PathBuf,
}

impl FileRunLock {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        Path::new("/run/core-ops").join("agent.lock")
    }
}

impl RunLock for FileRunLock {
    fn acquire(&self) -> Result<RunLockGuard, RunLockError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| RunLockError::Io(err.to_string()))?;
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    RunLockError::AlreadyHeld
                } else {
                    RunLockError::Io(err.to_string())
                }
            })?;
        let pid = std::process::id();
        writeln!(file, "pid={pid}").map_err(|err| RunLockError::Io(err.to_string()))?;
        Ok(RunLockGuard {
            lock_id: self.path.display().to_string(),
        })
    }

    fn release(&self, _guard: RunLockGuard) -> Result<(), RunLockError> {
        fs::remove_file(&self.path).map_err(|err| RunLockError::Io(err.to_string()))?;
        Ok(())
    }
}
