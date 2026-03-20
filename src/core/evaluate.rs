use crate::core::types::{EvaluatedArtifact, EvaluationInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutput {
    pub artifacts: Vec<EvaluatedArtifact>,
}

pub fn evaluate_desired_state(_input: &EvaluationInput) -> Result<EvaluationOutput, String> {
    Ok(EvaluationOutput { artifacts: Vec::new() })
}
