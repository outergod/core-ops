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
    let requested_ref = match classify_git_state(&canonical) {
        GitClassification::NotGit => "(stateless)".to_string(),
        GitClassification::Dirty => "(stateless+dirty)".to_string(),
        GitClassification::Clean(sha) => sha,
        GitClassification::ProbeFailed(reason) => {
            // Per `contracts/cli-flag.md` Error semantics: probe
            // failures (`git` missing, subprocess error, unexpected
            // non-zero exit downstream of a positive
            // `is-inside-work-tree`) fall back to `(stateless)`
            // BUT emit a stderr warning so operators don't
            // silently lose the distinction between an actually
            // non-git source and a degraded probe.
            eprintln!(
                "warning: git ref detection failed for {}: {}; recording as non-git source",
                canonical.display(),
                reason
            );
            "(stateless)".to_string()
        }
    };
    Ok(StatelessSource {
        repo_path: canonical,
        requested_repository,
        requested_ref,
    })
}

/// Outcome of probing the git state at a stateless `--source-repo`
/// path. Used by [`detect_provenance`] to map to the documented
/// `requested_ref` value (`(stateless)` / `(stateless+dirty)` /
/// 40-char SHA) plus emit a stderr warning when the probe degraded.
enum GitClassification {
    /// `git` ran successfully and confirmed the path is not inside a
    /// work tree. No warning emitted — this is the canonical
    /// non-git stateless source.
    NotGit,
    /// `git` ran successfully and the working tree is clean at the
    /// returned 40-char HEAD SHA.
    Clean(String),
    /// `git` ran successfully and the working tree has uncommitted
    /// changes (modified / added / deleted / untracked).
    Dirty,
    /// A git subprocess failed in a way that prevents classification
    /// (binary missing, unexpected non-zero exit downstream of a
    /// positive `is-inside-work-tree`, malformed output). Carries
    /// a short reason for the operator-facing warning. Per
    /// `contracts/cli-flag.md` the caller falls back to `(stateless)`.
    ProbeFailed(String),
}

fn classify_git_state(path: &Path) -> GitClassification {
    match is_inside_work_tree(path) {
        Ok(false) => GitClassification::NotGit,
        Err(reason) => GitClassification::ProbeFailed(reason),
        Ok(true) => match working_tree_clean(path) {
            Ok(false) => GitClassification::Dirty,
            Err(reason) => GitClassification::ProbeFailed(reason),
            Ok(true) => match head_sha(path) {
                Ok(sha) => GitClassification::Clean(sha),
                Err(reason) => GitClassification::ProbeFailed(reason),
            },
        },
    }
}

/// Probe whether `path` is inside a git work tree.
///
/// `Ok(true)`   — `git rev-parse --is-inside-work-tree` exited 0
///                with stdout `true`.
/// `Ok(false)`  — `git` ran successfully and definitively reported
///                "not a git repository" (the canonical non-git
///                case), or exit 0 with stdout `false`.
/// `Err(reason)` — the probe failed in a way that does NOT prove
///                `path` is not a git repo: the `git` binary failed
///                to spawn (missing from `$PATH`, fork error), or
///                git ran but exited non-zero for an unrecognized
///                reason (corrupt `.git/HEAD`, permission error
///                reading `.git/`, locked index, etc.). The caller
///                emits a stderr warning before falling back to
///                `(stateless)`.
///
/// The stderr content is inspected to keep the canonical
/// "not a git repository" path warning-free while surfacing the
/// damaged-repo case (which would otherwise masquerade as a clean
/// non-git directory).
fn is_inside_work_tree(path: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|err| format!("`git rev-parse --is-inside-work-tree` could not be spawned: {err}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim() == "true");
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The canonical non-git case prints "fatal: not a git repository
    // (or any parent up to ...)". Treat that as a definitive `Ok(false)`
    // — no probe-failure warning. Anything else (corrupt HEAD, locked
    // index, permission error inside `.git/`, etc.) is a probe failure.
    if stderr.contains("not a git repository") {
        return Ok(false);
    }
    Err(format!(
        "`git rev-parse --is-inside-work-tree` exited non-zero: {}",
        stderr.trim()
    ))
}

/// Probe whether the working tree at `path` is clean.
///
/// `Ok(true)`   — `git status --porcelain` succeeded with empty output.
/// `Ok(false)`  — `git status --porcelain` succeeded with non-empty output.
/// `Err(reason)` — subprocess failed unexpectedly (binary missing,
///                non-zero exit, etc.). Surfaced as a warning by
///                the caller; per `research.md` D1 step 5 we still
///                fall back to `(stateless)` so probe failure is
///                not conflated with an actually-dirty tree.
///
/// Passes `--untracked-files=normal` explicitly so the probe is
/// independent of `status.showUntrackedFiles` set anywhere in the
/// user's gitconfig levels. Without this override, a repo (or user)
/// with `status.showUntrackedFiles=no` would silently classify an
/// uncommitted authoring edit as clean and emit the parent commit's
/// SHA instead of `(stateless+dirty)` — exactly the operator state
/// stateless mode is meant to flag.
fn working_tree_clean(path: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "--",
            ".",
        ])
        .output()
        .map_err(|err| format!("`git status --porcelain` could not be spawned: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "`git status --porcelain` exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout.is_empty())
}

fn head_sha(path: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|err| format!("`git rev-parse HEAD` could not be spawned: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "`git rev-parse HEAD` exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(sha)
    } else {
        Err(format!(
            "`git rev-parse HEAD` returned unexpected output: {sha:?}"
        ))
    }
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
    fn dirty_detection_overrides_repo_show_untracked_files_no() {
        // Repo (or user) config can set `status.showUntrackedFiles=no`,
        // which would normally make `git status --porcelain` skip
        // untracked files. The probe MUST override that with
        // `--untracked-files=normal` so authoring edits in a
        // stateless source-repo are still classified dirty.
        let tmp = TempDir::new().expect("tempdir");
        run_git(tmp.path(), &["init", "-q"]);
        std::fs::write(tmp.path().join("README"), "fixture\n").expect("write");
        run_git(tmp.path(), &["add", "."]);
        run_git(tmp.path(), &["commit", "-q", "-m", "fixture"]);
        // Pin the regression: locally configure the repo to hide
        // untracked files. Without the `--untracked-files=normal`
        // override, the next probe would falsely report clean.
        run_git(
            tmp.path(),
            &["config", "--local", "status.showUntrackedFiles", "no"],
        );
        std::fs::write(tmp.path().join("scratch.txt"), "wip\n").expect("write");

        let result = detect_provenance(tmp.path()).expect("detect");
        assert_eq!(
            result.requested_ref, "(stateless+dirty)",
            "untracked files MUST be detected even when the repo \
             configures status.showUntrackedFiles=no"
        );
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
