use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::core::types::{
    Boundaries, BoundaryScope, DesiredState, Invariant, Workload,
};
use crate::io::quadlet::{read_quadlet_dir, QuadletError};

#[derive(Debug)]
pub enum RepoError {
    GitCloneFailed(String),
    GitFetchFailed(String),
    GitCheckoutFailed(String),
    InvalidRepoSource(String),
    MissingQuadletDir(PathBuf),
    Quadlet(QuadletError),
}

impl From<QuadletError> for RepoError {
    fn from(err: QuadletError) -> Self {
        RepoError::Quadlet(err)
    }
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoError::GitCloneFailed(msg) => write!(f, "git clone failed: {}", msg),
            RepoError::GitFetchFailed(msg) => write!(f, "git fetch failed: {}", msg),
            RepoError::GitCheckoutFailed(msg) => write!(f, "git checkout failed: {}", msg),
            RepoError::InvalidRepoSource(src) => write!(f, "invalid repo source: {}", src),
            RepoError::MissingQuadletDir(path) => {
                write!(f, "missing quadlet dir: {}", path.display())
            }
            RepoError::Quadlet(err) => write!(f, "quadlet error: {}", err),
        }
    }
}

impl std::error::Error for RepoError {}

pub fn load_desired_state(repo_source: &str, revision_id: &str) -> Result<DesiredState, RepoError> {
    let temp = TempDir::new().map_err(|err| RepoError::GitCloneFailed(err.to_string()))?;
    if looks_like_url(repo_source) {
        git_clone(repo_source, temp.path())?;
    } else {
        let repo_path = Path::new(repo_source);
        if !repo_path.exists() {
            return Err(RepoError::InvalidRepoSource(repo_source.to_string()));
        }
        git_clone(repo_source, temp.path())?;
    }

    git_fetch_revision(temp.path(), revision_id)?;
    git_checkout_revision(temp.path())?;

    let repo_path = temp.path().to_path_buf();
    let quadlet_dir = repo_path.join("quadlets");
    if !quadlet_dir.exists() {
        return Err(RepoError::MissingQuadletDir(quadlet_dir));
    }
    let workloads = read_quadlet_dir(&quadlet_dir)?;
    Ok(desired_state_from_workloads(
        &repo_path,
        revision_id,
        workloads,
    ))
}

pub fn desired_state_from_workloads(
    repo_path: &Path,
    revision_id: &str,
    workloads: Vec<Workload>,
) -> DesiredState {
    DesiredState {
        repository_ref: repo_path.display().to_string(),
        revision_id: revision_id.to_string(),
        workloads,
        invariants: vec![Invariant::BoundariesDeclared, Invariant::DeterministicPlan],
        boundaries: Boundaries {
            scopes: vec![BoundaryScope::QuadletSystemd],
        },
    }
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("file://")
        || value.starts_with("ssh://")
        || value.contains('@') && value.contains(':')
}

fn git_clone(repo: &str, dest: &Path) -> Result<(), RepoError> {
    let output = Command::new("git")
        .arg("clone")
        .arg("--no-checkout")
        .arg(repo)
        .arg(dest)
        .output()
        .map_err(|err| RepoError::GitCloneFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitCloneFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

fn git_fetch_revision(repo_path: &Path, revision: &str) -> Result<(), RepoError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("fetch")
        .arg("--depth")
        .arg("1")
        .arg("origin")
        .arg(revision)
        .output()
        .map_err(|err| RepoError::GitFetchFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitFetchFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

fn git_checkout_revision(repo_path: &Path) -> Result<(), RepoError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("checkout")
        .arg("--detach")
        .arg("FETCH_HEAD")
        .output()
        .map_err(|err| RepoError::GitCheckoutFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitCheckoutFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}
