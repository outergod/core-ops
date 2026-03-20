use crate::core::types::{EvaluatedArtifact, EvaluationInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutput {
    pub artifacts: Vec<EvaluatedArtifact>,
}

pub fn evaluate_desired_state(input: &EvaluationInput) -> Result<EvaluationOutput, String> {
    let mut artifacts = Vec::new();
    for service_name in &input.host.services {
        let service = input
            .catalog
            .services
            .get(service_name)
            .ok_or_else(|| format!("missing service: {}", service_name))?;
        for artifact in &service.artifacts {
            artifacts.push(EvaluatedArtifact {
                name: artifact.name.clone(),
                quadlet_type: artifact.quadlet_type.clone(),
                contents: artifact.contents.clone(),
                source_layers: vec![artifact.source_path.clone()],
            });
        }
    }
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(EvaluationOutput { artifacts })
}
