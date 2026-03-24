use std::collections::BTreeMap;
use std::path::Path;

use crate::core::errors::EvaluationError;
use crate::core::types::{
    ConfigFileSource, DropInSource, EvaluatedArtifact, EvaluatedConfigFile, EvaluatedDropIn,
    EvaluationInput, MountDependency, MountDeclaration, MountVerificationMode, PathDependencyMode,
    PreparedTargetPath, QuadletType, ServiceDependencyEdit, UnitDependencyMode,
};
use crate::core::unit::apply_service_mount_dependencies;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationOutput {
    pub artifacts: Vec<EvaluatedArtifact>,
    pub socket_dropins: Vec<EvaluatedDropIn>,
    pub config_files: Vec<EvaluatedConfigFile>,
    pub mount_declarations: Vec<MountDeclaration>,
    pub mount_dependencies: Vec<MountDependency>,
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

    let mount_declarations = collect_mount_declarations(input, &artifacts)?;
    let mount_dependencies = expand_mount_dependencies(input, &mount_declarations)?;
    let dependency_map: BTreeMap<&str, ServiceDependencyEdit> = mount_dependencies
        .iter()
        .map(|dependency| {
            let mut after_units = Vec::new();
            let mut requires_units = Vec::new();
            for mount_id in &dependency.mount_ids {
                let declaration = mount_declarations
                    .iter()
                    .find(|decl| decl.id == *mount_id)
                    .expect("mount dependency was expanded from known declaration");
                let explicit_units = explicit_dependency_units(declaration);
                after_units.extend(explicit_units.clone());
                requires_units.extend(explicit_units);
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

    for artifact in &mut artifacts {
        if !matches!(artifact.quadlet_type, QuadletType::Container | QuadletType::Pod) {
            continue;
        }
        if let Some(service_name) = service_name_from_layers(&artifact.source_layers) {
            if let Some(edit) = dependency_map.get(service_name.as_str()) {
                artifact.contents = apply_service_mount_dependencies(&artifact.contents, edit);
            }
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

fn explicit_dependency_units(declaration: &MountDeclaration) -> Vec<String> {
    match declaration.automount_unit_name() {
        Some(automount_unit) => vec![automount_unit, declaration.mount_unit_name()],
        None => vec![declaration.mount_unit_name()],
    }
}

fn collect_mount_declarations(
    input: &EvaluationInput,
    artifacts: &[EvaluatedArtifact],
) -> Result<Vec<MountDeclaration>, EvaluationError> {
    let mut by_id: BTreeMap<String, MountDeclaration> = input
        .host
        .services
        .iter()
        .filter_map(|service_name| input.catalog.services.get(service_name))
        .flat_map(|service| service.mount_declarations.iter().cloned())
        .map(|mount| (mount.id.clone(), mount))
        .collect();

    for declaration in collect_mount_declarations_from_artifacts(artifacts)? {
        by_id.insert(declaration.id.clone(), declaration);
    }

    for override_mount in &input.overlays.mount_overrides {
        by_id.insert(override_mount.id.clone(), override_mount.clone());
    }

    let mut mounts: Vec<MountDeclaration> = by_id.into_values().collect();
    mounts.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(mounts)
}

fn collect_mount_declarations_from_artifacts(
    artifacts: &[EvaluatedArtifact],
) -> Result<Vec<MountDeclaration>, EvaluationError> {
    let mut mounts_by_stem = BTreeMap::new();
    let mut automounts_by_stem = BTreeMap::new();

    for artifact in artifacts {
        match artifact.quadlet_type {
            QuadletType::Mount => {
                mounts_by_stem.insert(unit_stem(&artifact.name), artifact);
            }
            QuadletType::Automount => {
                automounts_by_stem.insert(unit_stem(&artifact.name), artifact);
            }
            _ => {}
        }
    }

    let mut declarations = Vec::new();
    for (stem, mount_artifact) in mounts_by_stem {
        let parsed_mount = ParsedManagedMount::from_mount_artifact(mount_artifact)?;
        let Some(parsed_mount) = parsed_mount else {
            continue;
        };
        let automount = automounts_by_stem.remove(&stem);
        let declaration = parsed_mount.into_declaration(automount)?;
        declarations.push(declaration);
    }

    for (stem, artifact) in automounts_by_stem {
        let section = parse_sections(&artifact.contents);
        if section_value(&section, "X-CoreOps", "Id").is_some() {
            return Err(EvaluationError::new(format!(
                "managed automount artifact requires matching mount artifact: {}",
                stem
            )));
        }
    }

    declarations.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(declarations)
}

fn expand_mount_dependencies(
    input: &EvaluationInput,
    declarations: &[MountDeclaration],
) -> Result<Vec<MountDependency>, EvaluationError> {
    let declaration_map: BTreeMap<&str, &MountDeclaration> = declarations
        .iter()
        .map(|decl| (decl.id.as_str(), decl))
        .collect();
    let mut dependencies = Vec::new();
    for service_name in &input.host.services {
        let Some(service) = input.catalog.services.get(service_name) else {
            continue;
        };
        let mount_ids = input
            .overlays
            .service_mount_overrides
            .get(service_name)
            .cloned()
            .unwrap_or_else(|| service.service_mounts.clone());
        if mount_ids.is_empty() {
            continue;
        }
        let mut consumed_paths = Vec::new();
        for mount_id in &mount_ids {
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
            mount_ids,
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
    let mut map: BTreeMap<String, EvaluatedConfigFile> = BTreeMap::new();
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

fn service_name_from_layers(layers: &[String]) -> Option<String> {
    for layer in layers {
        let marker = "/services/";
        let start = layer.find(marker)? + marker.len();
        let tail = &layer[start..];
        let service = tail.split('/').next()?;
        if !service.is_empty() {
            return Some(service.to_string());
        }
    }
    None
}

fn unit_stem(unit_name: &str) -> String {
    Path::new(unit_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(unit_name)
        .to_string()
}

struct ParsedManagedMount {
    id: String,
    target_path: String,
    source: String,
    fstype: String,
    mount_options: Vec<String>,
    network_backed: bool,
    verification_mode: MountVerificationMode,
    ownership_scope: Vec<String>,
    prepared_path: Option<PreparedTargetPath>,
}

impl ParsedManagedMount {
    fn from_mount_artifact(artifact: &EvaluatedArtifact) -> Result<Option<Self>, EvaluationError> {
        let sections = parse_sections(&artifact.contents);
        let Some(id) = section_value(&sections, "X-CoreOps", "Id") else {
            return Ok(None);
        };
        let target_path = required_section_value(&sections, "Mount", "Where", &artifact.name)?;
        let source = required_section_value(&sections, "Mount", "What", &artifact.name)?;
        let fstype = required_section_value(&sections, "Mount", "Type", &artifact.name)?;
        let mount_options = section_value(&sections, "Mount", "Options")
            .map(|value| split_csv(&value))
            .unwrap_or_default();
        let network_backed = section_bool_value(&sections, "X-CoreOps", "NetworkBacked")
            .unwrap_or_else(|| is_network_fstype(&fstype));
        let verification_mode = parse_verification_mode(
            section_value(&sections, "X-CoreOps", "VerificationMode").as_deref(),
            &artifact.name,
        )?;
        let ownership_scope = section_value(&sections, "X-CoreOps", "OwnershipScope")
            .map(|value| split_csv(&value))
            .unwrap_or_default();
        let prepared_path = parse_prepared_path(&sections, &target_path, &artifact.name)?;

        if let Some(policy) = section_value(&sections, "X-CoreOps", "RemovalPolicy") {
            let normalized = policy.trim().to_ascii_lowercase();
            if normalized != "managed" {
                return Err(EvaluationError::new(format!(
                    "unsupported X-CoreOps RemovalPolicy in {}: {}",
                    artifact.name, policy
                )));
            }
        }

        Ok(Some(Self {
            id,
            target_path,
            source,
            fstype,
            mount_options,
            network_backed,
            verification_mode,
            ownership_scope,
            prepared_path,
        }))
    }

    fn into_declaration(
        self,
        automount_artifact: Option<&EvaluatedArtifact>,
    ) -> Result<MountDeclaration, EvaluationError> {
        let automount = match automount_artifact {
            Some(artifact) => {
                let sections = parse_sections(&artifact.contents);
                let where_path = required_section_value(&sections, "Automount", "Where", &artifact.name)?;
                if where_path != self.target_path {
                    return Err(EvaluationError::new(format!(
                        "automount Where does not match mount target: {} != {}",
                        where_path, self.target_path
                    )));
                }
                if let Some(id) = section_value(&sections, "X-CoreOps", "Id") {
                    if id != self.id {
                        return Err(EvaluationError::new(format!(
                            "automount X-CoreOps Id does not match mount artifact: {} != {}",
                            id, self.id
                        )));
                    }
                }
                true
            }
            None => false,
        };

        Ok(MountDeclaration {
            id: self.id,
            target_path: self.target_path,
            source: self.source,
            fstype: self.fstype,
            mount_options: self.mount_options,
            network_backed: self.network_backed,
            automount,
            verification_mode: self.verification_mode,
            ownership_scope: self.ownership_scope,
            prepared_path: self.prepared_path,
        })
    }
}

fn parse_sections(contents: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut current = String::new();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].trim().to_string();
            sections.entry(current.clone()).or_default();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if current.is_empty() {
            continue;
        }
        sections
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }
    sections
}

fn required_section_value(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    key: &str,
    unit_name: &str,
) -> Result<String, EvaluationError> {
    section_value(sections, section, key).ok_or_else(|| {
        EvaluationError::new(format!(
            "missing {} {}= in managed mount artifact {}",
            section, key, unit_name
        ))
    })
}

fn section_value(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    key: &str,
) -> Option<String> {
    sections.get(section).and_then(|values| values.get(key)).cloned()
}

fn section_bool_value(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    section: &str,
    key: &str,
) -> Option<bool> {
    let value = section_value(sections, section, key)?;
    let normalized = value.trim().to_ascii_lowercase();
    Some(matches!(normalized.as_str(), "1" | "yes" | "true" | "on"))
}

fn parse_prepared_path(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    target_path: &str,
    unit_name: &str,
) -> Result<Option<PreparedTargetPath>, EvaluationError> {
    let prepared_path = section_value(sections, "X-CoreOps", "PreparedPath");
    let create_if_missing = section_bool_value(sections, "X-CoreOps", "PreparedCreateIfMissing");
    let owner = section_value(sections, "X-CoreOps", "PreparedOwner");
    let group = section_value(sections, "X-CoreOps", "PreparedGroup");
    let mode = section_value(sections, "X-CoreOps", "PreparedMode");
    let service_consumed = section_bool_value(sections, "X-CoreOps", "PreparedServiceConsumed");

    if prepared_path.is_none()
        && create_if_missing.is_none()
        && owner.is_none()
        && group.is_none()
        && mode.is_none()
        && service_consumed.is_none()
    {
        return Ok(None);
    }

    let path = prepared_path.unwrap_or_else(|| target_path.to_string());
    if path != target_path {
        return Err(EvaluationError::new(format!(
            "X-CoreOps PreparedPath must match Mount Where in {}",
            unit_name
        )));
    }

    Ok(Some(PreparedTargetPath {
        path,
        create_if_missing: create_if_missing.unwrap_or(true),
        owner,
        group,
        mode,
        service_consumed: service_consumed.unwrap_or(false),
    }))
}

fn parse_verification_mode(
    value: Option<&str>,
    unit_name: &str,
) -> Result<MountVerificationMode, EvaluationError> {
    let Some(value) = value else {
        return Ok(MountVerificationMode::UnitAndPath);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "unitandpath" | "unit_and_path" => Ok(MountVerificationMode::UnitAndPath),
        other => Err(EvaluationError::new(format!(
            "unsupported X-CoreOps VerificationMode in {}: {}",
            unit_name, other
        ))),
    }
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_network_fstype(fstype: &str) -> bool {
    matches!(
        fstype.trim().to_ascii_lowercase().as_str(),
        "nfs" | "nfs4" | "cifs" | "smbfs" | "sshfs" | "glusterfs" | "ceph" | "cephfs"
    )
}
