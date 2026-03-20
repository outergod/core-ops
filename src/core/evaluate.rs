use crate::core::errors::EvaluationError;
use crate::core::types::{
    DropInSource, EvaluatedArtifact, EvaluatedDropIn, EvaluationInput, QuadletType,
};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutput {
    pub artifacts: Vec<EvaluatedArtifact>,
    pub socket_dropins: Vec<EvaluatedDropIn>,
}

pub fn evaluate_desired_state(input: &EvaluationInput) -> Result<EvaluationOutput, EvaluationError> {
    let mut artifacts = Vec::new();
    let mut socket_dropins = Vec::new();
    for service_name in &input.host.services {
        let service = input
            .catalog
            .services
            .get(service_name)
            .ok_or_else(|| EvaluationError::new(format!("missing service: {}", service_name)))?;
        for artifact in &service.artifacts {
            let mut contents = artifact.contents.clone();
            let mut source_layers = vec![artifact.source_path.clone()];

            let base_dropins = collect_dropins(&service.base_dropins, &artifact.name);
            let host_dropins = collect_dropins(&input.overlays.overrides, &artifact.name);

            if artifact.quadlet_type == QuadletType::Socket {
                socket_dropins.extend(to_socket_dropins(&artifact.name, &base_dropins));
                socket_dropins.extend(to_socket_dropins(&artifact.name, &host_dropins));
            } else {
                apply_dropins(&mut contents, &mut source_layers, &base_dropins);
                apply_dropins(&mut contents, &mut source_layers, &host_dropins);
            }

            artifacts.push(EvaluatedArtifact {
                name: artifact.name.clone(),
                quadlet_type: artifact.quadlet_type.clone(),
                contents,
                source_layers,
            });
        }
    }
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    socket_dropins.sort_by(|a, b| {
        (a.target.clone(), a.file_name.clone()).cmp(&(b.target.clone(), b.file_name.clone()))
    });
    Ok(EvaluationOutput {
        artifacts,
        socket_dropins,
    })
}

fn collect_dropins(dropins: &[DropInSource], target: &str) -> Vec<DropInSource> {
    let mut matches: Vec<DropInSource> = dropins
        .iter()
        .filter(|dropin| dropin.target == target)
        .cloned()
        .collect();
    matches.sort_by(|a, b| dropin_order_key(&a.source_path).cmp(&dropin_order_key(&b.source_path)));
    matches
}

fn apply_dropins(contents: &mut String, sources: &mut Vec<String>, dropins: &[DropInSource]) {
    for dropin in dropins {
        if !contents.ends_with('\n') {
            contents.push('\n');
        }
        contents.push_str(&dropin.contents);
        sources.push(dropin.source_path.clone());
    }
}

fn dropin_order_key(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn to_socket_dropins(target: &str, dropins: &[DropInSource]) -> Vec<EvaluatedDropIn> {
    dropins
        .iter()
        .map(|dropin| EvaluatedDropIn {
            target: target.to_string(),
            file_name: dropin_file_name(&dropin.source_path),
            contents: dropin.contents.clone(),
            source_path: dropin.source_path.clone(),
        })
        .collect()
}

fn dropin_file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}
