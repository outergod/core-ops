//! Stateless `--source-repo` provenance detection (spec/017).
//!
//! Used by `core-ops plan/apply/explain --source-repo <PATH>` to bypass
//! the persisted controller configuration written by `core-ops init` and
//! source desired state directly from a filesystem directory.
//!
//! Records path-based provenance per FR-013 + 2026-05-05 clarification Q3:
//!
//! | Source path state                              | `requested_ref`         |
//! |------------------------------------------------|-------------------------|
//! | Non-git directory                              | `(stateless)`           |
//! | Git working tree, dirty (`status --porcelain`) | `(stateless+dirty)`     |
//! | Git working tree, clean at HEAD                | `<full 40-char SHA>`    |
//!
//! Sentinels begin with `(`, which is invalid in a git ref name per
//! `git check-ref-format`, so they cannot collide with real refs.
//!
//! Implementation shells out to the `git` binary via `std::process::Command`,
//! mirroring the established pattern at `src/cli/init.rs` and
//! `src/io/repo.rs`. No new runtime dependency on `git2` or similar.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Path-based source-of-truth identifier carrying provenance for
/// stateless mode. Constructed by [`detect_provenance`].
#[derive(Clone, Debug)]
pub struct StatelessSource {
    /// Canonicalized, symlink-resolved absolute path to the source-repo.
    pub repo_path: PathBuf,
    /// Stringified `repo_path`, recorded as `desired_state.repository`
    /// in audit + provenance. Always begins with `/` so it is
    /// unambiguously distinguishable from a git URL (which contains
    /// `:` for `https://` or `user@host:`).
    pub requested_repository: String,
    /// Either a full 40-char SHA hex (clean git checkout at HEAD),
    /// `(stateless+dirty)` (dirty git working tree), or `(stateless)`
    /// (non-git directory). Recorded as `desired_state.requested_ref`
    /// in audit + provenance. Sentinels disambiguated by the leading
    /// `(` character.
    pub requested_ref: String,
}

/// Errors surfaced from path-shaped validation in stateless mode.
/// Layout/parser errors continue to bubble up via `RepoError` and
/// surface with their existing exit-code mapping.
#[derive(Debug)]
pub enum SourceRefError {
    /// `--source-repo <PATH>` does not exist on the filesystem
    /// (`std::io::ErrorKind::NotFound` from `fs::metadata`). Mapped
    /// to exit code 64 (`EX_USAGE`) per `contracts/cli-flag.md`.
    PathMissing(PathBuf),
    /// `--source-repo <PATH>` exists but is not a directory.
    /// Mapped to exit code 64 (`EX_USAGE`).
    PathNotDirectory(PathBuf),
    /// Path metadata inspection failed for a non-`NotFound` reason
    /// (typically `PermissionDenied` or other I/O error). Distinct
    /// from `PathMissing` so automation can tell "directory does
    /// not exist" from "directory exists but the controller cannot
    /// inspect it". Mapped to exit code 66.
    PathInaccessible { path: PathBuf, source: std::io::Error },
    /// Path canonicalization failed (e.g., symlink loop, insufficient
    /// permissions on an intermediate component). Mapped to exit
    /// code 66.
    Canonicalize { path: PathBuf, source: std::io::Error },
}

impl SourceRefError {
    /// Process exit code per `contracts/cli-flag.md` Error semantics
    /// table. 64 = `EX_USAGE`, 65 = `EX_DATAERR`, 66 = path-shape.
    pub fn exit_code(&self) -> i32 {
        match self {
            SourceRefError::PathMissing(_) | SourceRefError::PathNotDirectory(_) => 64,
            SourceRefError::PathInaccessible { .. } | SourceRefError::Canonicalize { .. } => 66,
        }
    }
}

impl std::fmt::Display for SourceRefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceRefError::PathMissing(path) => {
                write!(f, "--source-repo path does not exist: {}", path.display())
            }
            SourceRefError::PathNotDirectory(path) => {
                write!(f, "--source-repo path is not a directory: {}", path.display())
            }
            SourceRefError::PathInaccessible { path, source } => write!(
                f,
                "--source-repo path could not be accessed: {}: {}",
                path.display(),
                source
            ),
            SourceRefError::Canonicalize { path, source } => write!(
                f,
                "--source-repo path could not be canonicalized: {}: {}",
                path.display(),
                source
            ),
        }
    }
}

impl std::error::Error for SourceRefError {}

/// Detect path-based provenance for the stateless `--source-repo` flag.
///
/// Validates `path` is an existing directory, canonicalizes it, then
/// classifies its git state. Returns a [`StatelessSource`] carrying
/// the canonical path and the resolved `requested_ref` value.
///
/// Uses `fs::metadata` rather than `Path::exists()` / `Path::is_dir()`
/// so I/O errors (most commonly `PermissionDenied`) surface as
/// `PathInaccessible` instead of being collapsed to `PathMissing`.
/// Automation that distinguishes "does not exist" from "exists but
/// inaccessible" reads the documented exit code (64 vs 66).
///
/// See module-level docs for the git-state classification table.
pub fn detect_provenance(path: &Path) -> Result<StatelessSource, SourceRefError> {
    let metadata = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(SourceRefError::PathMissing(path.to_path_buf()));
        }
        Err(err) => {
            return Err(SourceRefError::PathInaccessible {
                path: path.to_path_buf(),
                source: err,
            });
        }
    };
    if !metadata.is_dir() {
        return Err(SourceRefError::PathNotDirectory(path.to_path_buf()));
    }
    let canonical = std::fs::canonicalize(path).map_err(|err| SourceRefError::Canonicalize {
        path: path.to_path_buf(),
        source: err,
    })?;
    let requested_repository = canonical.to_string_lossy().into_owned();
    let requested_ref = if is_inside_work_tree(&canonical) {
        // Per research.md D1 step 5: on any subprocess error, fall
        // back to `(stateless)`. We distinguish three states:
        //   Some(true)  → working tree definitely dirty
        //   Some(false) → working tree definitely clean
        //   None        → could not determine; treat as non-verifiable
        match working_tree_clean(&canonical) {
            Some(false) => "(stateless+dirty)".to_string(),
            Some(true) => head_sha(&canonical).unwrap_or_else(|| "(stateless)".to_string()),
            None => "(stateless)".to_string(),
        }
    } else {
        "(stateless)".to_string()
    };
    Ok(StatelessSource {
        repo_path: canonical,
        requested_repository,
        requested_ref,
    })
}

fn is_inside_work_tree(path: &Path) -> bool {
    match Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
    {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim() == "true"
        }
        _ => false,
    }
}

/// Probe whether the working tree at `path` is clean.
///
/// `Some(true)`  — `git status --porcelain` succeeded with empty output.
/// `Some(false)` — `git status --porcelain` succeeded with non-empty output.
/// `None`        — subprocess failed (git missing, invocation error, or
///                non-zero exit); the caller falls back to `(stateless)`
///                per `research.md` D1 step 5 so probe failure is not
///                conflated with an actually-dirty tree in the audit chain.
fn working_tree_clean(path: &Path) -> Option<bool> {
    match Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["status", "--porcelain", "--", "."])
        .output()
    {
        Ok(out) if out.status.success() => Some(out.stdout.is_empty()),
        _ => None,
    }
}

fn head_sha(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit())).then_some(sha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as ProcessCommand;
    use tempfile::TempDir;

    fn run_git(repo: &Path, args: &[&str]) {
        let status = ProcessCommand::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
            .status()
            .expect("git invocation");
        assert!(status.success(), "git {:?} failed", args);
    }

    #[test]
    fn non_git_directory_records_stateless_sentinel() {
        let tmp = TempDir::new().expect("tempdir");
        let result = detect_provenance(tmp.path()).expect("detect");
        assert_eq!(result.requested_ref, "(stateless)");
        assert_eq!(
            result.repo_path,
            std::fs::canonicalize(tmp.path()).unwrap()
        );
        assert!(result.requested_repository.starts_with('/'));
    }

    #[test]
    fn clean_git_checkout_records_full_sha() {
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join("README"), "fixture\n").expect("write");
        run_git(tmp.path(), &["add", "."]);
        run_git(tmp.path(), &["commit", "-q", "-m", "fixture"]);

        let result = detect_provenance(tmp.path()).expect("detect");
        assert_eq!(result.requested_ref.len(), 40);
        assert!(result.requested_ref.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn dirty_working_tree_records_stateless_dirty_sentinel() {
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join("README"), "fixture\n").expect("write");
        run_git(tmp.path(), &["add", "."]);
        run_git(tmp.path(), &["commit", "-q", "-m", "fixture"]);
        // Untracked file → status --porcelain is non-empty.
        std::fs::write(tmp.path().join("scratch.txt"), "wip\n").expect("write");

        let result = detect_provenance(tmp.path()).expect("detect");
        assert_eq!(result.requested_ref, "(stateless+dirty)");
    }

    #[test]
    fn missing_path_yields_path_missing_error() {
        let tmp = TempDir::new().expect("tempdir");
        let nonexistent = tmp.path().join("does-not-exist");
        let err = detect_provenance(&nonexistent).expect_err("missing path");
        match err {
            SourceRefError::PathMissing(_) => {}
            other => panic!("expected PathMissing, got {other:?}"),
        }
        assert_eq!(
            detect_provenance(&nonexistent).unwrap_err().exit_code(),
            64
        );
    }

    #[test]
    fn file_path_yields_path_not_directory_error() {
        let tmp = TempDir::new().expect("tempdir");
        let file_path = tmp.path().join("a-file");
        std::fs::write(&file_path, "x").expect("write");
        let err = detect_provenance(&file_path).expect_err("not directory");
        match err {
            SourceRefError::PathNotDirectory(_) => {}
            other => panic!("expected PathNotDirectory, got {other:?}"),
        }
        assert_eq!(detect_provenance(&file_path).unwrap_err().exit_code(), 64);
    }

    #[test]
    fn sentinels_cannot_collide_with_real_git_refs() {
        // `(` is reserved per `git check-ref-format`.
        for sentinel in ["(stateless)", "(stateless+dirty)"] {
            let status = ProcessCommand::new("git")
                .args(["check-ref-format", "--", sentinel])
                .status();
            // Either git rejects it or the binary is missing; both are
            // acceptable — the invariant is that sentinels are not
            // valid refs that consumers might mistake for SHAs.
            if let Ok(status) = status {
                assert!(
                    !status.success(),
                    "sentinel {sentinel} unexpectedly accepted as a git ref"
                );
            }
        }
    }
}
