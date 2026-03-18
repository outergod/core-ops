use std::path::{Path, PathBuf};

use crate::core::types::{
    Boundaries, BoundaryScope, DesiredState, Invariant, Workload,
};
use crate::io::quadlet::{read_quadlet_dir, QuadletError};

#[derive(Debug)]
pub enum RepoError {
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
            RepoError::MissingQuadletDir(path) => {
                write!(f, "missing quadlet dir: {}", path.display())
            }
            RepoError::Quadlet(err) => write!(f, "quadlet error: {}", err),
        }
    }
}

impl std::error::Error for RepoError {}

pub fn load_desired_state(repo_path: &Path, revision_id: &str) -> Result<DesiredState, RepoError> {
    let quadlet_dir = repo_path.join("quadlets");
    if !quadlet_dir.exists() {
        return Err(RepoError::MissingQuadletDir(quadlet_dir));
    }

    let workloads = read_quadlet_dir(&quadlet_dir)?;
    Ok(desired_state_from_workloads(
        repo_path,
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
