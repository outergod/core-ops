use std::path::Path;

use crate::core::types::{ObservedState, Workload};
use crate::io::quadlet::{read_quadlet_dir, QuadletError};

#[derive(Debug)]
pub enum ObservedError {
    Quadlet(QuadletError),
}

impl From<QuadletError> for ObservedError {
    fn from(err: QuadletError) -> Self {
        ObservedError::Quadlet(err)
    }
}

impl std::fmt::Display for ObservedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservedError::Quadlet(err) => write!(f, "observed state error: {}", err),
        }
    }
}

impl std::error::Error for ObservedError {}

pub fn read_observed_state(
    quadlet_dir: &Path,
    observed_revision_id: Option<String>,
) -> Result<ObservedState, ObservedError> {
    let workloads: Vec<Workload> = if quadlet_dir.exists() {
        read_quadlet_dir(quadlet_dir)?
    } else {
        Vec::new()
    };

    Ok(ObservedState {
        observed_revision_id,
        units: Vec::new(),
        workloads,
        last_reconcile_id: None,
        host_info: None,
    })
}
