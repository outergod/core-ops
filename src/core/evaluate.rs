use crate::core::errors::EvaluationError;
use crate::core::types::{
    ConfigFileSource, DropInSource, EvaluatedArtifact, EvaluatedConfigFile, EvaluatedDropIn,
    EvaluationInput, MountDependency, MountDeclaration, PathDependencyMode, QuadletType,
    ServiceDependencyEdit, UnitDependencyMode,
};
use crate::core::unit::apply_service_mount_dependencies;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutput {
    pub artifacts: Vec<EvaluatedArtifact>,
    pub socket_dropins: Vec<EvaluatedDropIn>,
    pub config_files: Vec<EvaluatedConfigFile>,
    pub mount_declarations: Vec<MountDeclaration>,
    pub mount_dependencies: Vec<MountDependency>,
}

pub fn evaluate_desired_state(input: &EvaluationInput) -> Result<EvaluationOutput, EvaluationError> {
    let mount_declarations = collect_mount_declarations(input);
    let mount_dependencies = expand_mount_dependencies(input, &mount_declarations)?;
    let dependency_map: std::collections::BTreeMap<&str, ServiceDependencyEdit> = mount_dependencies
        .iter()
        .map(|dependency| {
            let mut after_units = Vec::new();
            let mut requires_units = Vec::new();
            for mount_id in &dependency.mount_ids {
                let declaration = mount_declarations
                    .iter()
                    .find(|decl| decl.id == *mount_id)
                    .expect("mount dependency was expanded from known declaration");
                if let Some(automount) = declaration.automount_unit_name() {
                    after_units.push(automount);
                    requires_units.push(declaration.mount_unit_name());
                } else {
                    let mount_unit = declaration.mount_unit_name();
                    after_units.push(mount_unit.clone());
                    requires_units.push(mount_unit);
                }
            }
            (
                dependency.service_name.as_str(),
                ServiceDependencyEdit {
                    service_name: dependency.service_name.clone(),
                    requires_mounts_for: dependency.consumed_paths.clone(),
                    after_units,
                    requires_units,
                },
            )
        })
        .collect();
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
                if matches!(artifact.quadlet_type, QuadletType::Container | QuadletType::Pod) {
                    if let Some(edit) = dependency_map.get(service_name.as_str()) {
                        contents = apply_service_mount_dependencies(&contents, edit);
                    }
                }
            }

            artifacts.push(EvaluatedArtifact {
                name: artifact.name.clone(),
                quadlet_type: artifact.quadlet_type.clone(),
                contents,
                source_layers,
            });
        }

    }
    let base_configs: Vec<ConfigFileSource> = input
        .host
        .services
        .iter()
        .filter_map(|service_name| input.catalog.services.get(service_name))
        .flat_map(|service| service.config_files.iter().cloned())
        .collect();
    let host_configs = input
        .overlays
        .config_overrides
        .iter()
        .filter(|cfg| cfg.target_path.starts_with("/etc/"))
        .cloned()
        .collect::<Vec<_>>();
    let config_files = overlay_config_files(&base_configs, &host_configs);
    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    socket_dropins.sort_by(|a, b| {
        (a.target.clone(), a.file_name.clone()).cmp(&(b.target.clone(), b.file_name.clone()))
    });
    Ok(EvaluationOutput {
        artifacts,
        socket_dropins,
        config_files,
        mount_declarations,
        mount_dependencies,
    })
}

fn collect_mount_declarations(input: &EvaluationInput) -> Vec<MountDeclaration> {
    let mut mounts = Vec::new();
    for service_name in &input.host.services {
        if let Some(service) = input.catalog.services.get(service_name) {
            mounts.extend(service.mount_declarations.iter().cloned());
        }
    }
    mounts.sort_by(|a, b| a.id.cmp(&b.id));
    mounts
}

fn expand_mount_dependencies(
    input: &EvaluationInput,
    declarations: &[MountDeclaration],
) -> Result<Vec<MountDependency>, EvaluationError> {
    let declaration_map: std::collections::BTreeMap<&str, &MountDeclaration> = declarations
        .iter()
        .map(|decl| (decl.id.as_str(), decl))
        .collect();
    let mut dependencies = Vec::new();
    for service_name in &input.host.services {
        let Some(service) = input.catalog.services.get(service_name) else {
            continue;
        };
        if service.service_mounts.is_empty() {
            continue;
        }
        let mut consumed_paths = Vec::new();
        for mount_id in &service.service_mounts {
            let declaration = declaration_map.get(mount_id.as_str()).ok_or_else(|| {
                EvaluationError::new(format!(
                    "service {} references missing mount declaration {}",
                    service_name, mount_id
                ))
            })?;
            consumed_paths.push(declaration.target_path.clone());
        }
        consumed_paths.sort();
        consumed_paths.dedup();
        dependencies.push(MountDependency {
            service_name: service_name.clone(),
            mount_ids: service.service_mounts.clone(),
            consumed_paths,
            path_dependency_mode: PathDependencyMode::RequiresMountsFor,
            unit_dependency_mode: UnitDependencyMode::AfterAndRequires,
        });
    }
    dependencies.sort_by(|a, b| a.service_name.cmp(&b.service_name));
    Ok(dependencies)
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

fn overlay_config_files(
    base_files: &[ConfigFileSource],
    host_files: &[ConfigFileSource],
) -> Vec<EvaluatedConfigFile> {
    let mut map: std::collections::BTreeMap<String, EvaluatedConfigFile> = std::collections::BTreeMap::new();
    for cfg in base_files {
        map.insert(
            cfg.target_path.clone(),
            EvaluatedConfigFile {
                target_path: cfg.target_path.clone(),
                contents: cfg.contents.clone(),
                source_layers: vec![cfg.source_path.clone()],
            },
        );
    }
    for cfg in host_files {
        map.insert(
            cfg.target_path.clone(),
            EvaluatedConfigFile {
                target_path: cfg.target_path.clone(),
                contents: cfg.contents.clone(),
                source_layers: vec![cfg.source_path.clone()],
            },
        );
    }
    map.into_values().collect()
}
