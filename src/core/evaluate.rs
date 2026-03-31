use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::core::errors::EvaluationError;
use crate::core::types::{
    automount_unit_name_for_path, mount_unit_name_for_path, ConfigFileSource, DropInSource,
    EvaluatedArtifact, EvaluatedConfigFile, EvaluatedDropIn, EvaluationInput, ManagedObjectKind,
    MountDeclaration, MountDependency, MountVerificationMode, NormalizedManagedObject,
    NormalizedSnapshot, PathDependencyMode, PreparedTargetPath, QuadletType, ServiceDependencyEdit,
    UnitDependencyMode,
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

pub fn build_desired_snapshot(
    desired_revision_id: &str,
    scope_id: &str,
    evaluation: &EvaluationOutput,
) -> NormalizedSnapshot {
    let mut objects = Vec::new();
    for artifact in &evaluation.artifacts {
        let mut material_fields = BTreeMap::new();
        material_fields.insert("unit_name".to_string(), artifact.name.clone());
        material_fields.insert(
            "quadlet_type".to_string(),
            format!("{:?}", artifact.quadlet_type).to_lowercase(),
        );
        objects.push(NormalizedManagedObject {
            object_id: artifact.name.clone(),
            object_kind: desired_object_kind(&artifact.quadlet_type),
            material_fields,
            dependency_refs: Vec::new(),
        });
    }
    for config in &evaluation.config_files {
        let mut material_fields = BTreeMap::new();
        material_fields.insert("target_path".to_string(), config.target_path.clone());
        objects.push(NormalizedManagedObject {
            object_id: config.target_path.clone(),
            object_kind: ManagedObjectKind::RenderedArtifact,
            material_fields,
            dependency_refs: Vec::new(),
        });
    }
    objects.sort_by(|a, b| a.object_id.cmp(&b.object_id));
    NormalizedSnapshot {
        revision_id: Some(desired_revision_id.to_string()),
        scope_id: scope_id.to_string(),
        objects,
    }
}

pub fn build_desired_snapshot_from_state(
    desired: &crate::core::types::DesiredState,
    scope_id: &str,
) -> NormalizedSnapshot {
    let mut objects = desired
        .workloads
        .iter()
        .map(|workload| {
            let mut material_fields = BTreeMap::new();
            material_fields.insert("name".to_string(), workload.name.clone());
            material_fields.insert("unit_name".to_string(), workload.systemd_unit_name.clone());
            material_fields.insert(
                "quadlet_type".to_string(),
                format!("{:?}", workload.quadlet_type).to_lowercase(),
            );
            material_fields.insert("contents".to_string(), workload.quadlet_contents.clone());
            material_fields.insert(
                "enabled_state".to_string(),
                format!("{:?}", workload.enabled_state).to_lowercase(),
            );
            material_fields.insert(
                "restart_policy".to_string(),
                format!("{:?}", workload.restart_policy).to_lowercase(),
            );
            NormalizedManagedObject {
                object_id: workload.systemd_unit_name.clone(),
                object_kind: desired_object_kind(&workload.quadlet_type),
                material_fields,
                dependency_refs: dependency_refs_for_workload_state(desired, workload),
            }
        })
        .collect::<Vec<_>>();
    objects.sort_by(|a, b| a.object_id.cmp(&b.object_id));
    NormalizedSnapshot {
        revision_id: Some(desired.revision_id.clone()),
        scope_id: scope_id.to_string(),
        objects,
    }
}

pub fn dependency_refs_for_workload_state(
    desired: &crate::core::types::DesiredState,
    workload: &crate::core::types::Workload,
) -> Vec<String> {
    let mount_units_by_id: BTreeMap<&str, Vec<String>> = desired
        .mount_declarations
        .iter()
        .map(|mount| {
            let mut refs = vec![mount.mount_unit_name()];
            if let Some(automount) = mount.automount_unit_name() {
                refs.push(automount);
            }
            (mount.id.as_str(), refs)
        })
        .collect();
    let mut refs = desired
        .mount_dependencies
        .iter()
        .find(|dependency| dependency.service_name == workload.name)
        .map(|dependency| {
            dependency
                .mount_ids
                .iter()
                .flat_map(|mount_id| {
                    mount_units_by_id
                        .get(mount_id.as_str())
                        .cloned()
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let workload_ids = desired
        .workloads
        .iter()
        .map(|item| item.systemd_unit_name.as_str())
        .collect::<BTreeSet<_>>();
    let managed_configs = desired
        .managed_config_paths
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for raw_line in workload.quadlet_contents.lines() {
        let line = raw_line.trim();
        if let Some(value) = directive_value(line, "EnvironmentFile") {
            refs.extend(config_refs_for_path(value, &managed_configs));
        } else if let Some(value) = directive_value(line, "Volume") {
            if let Some(source) = value.split(':').next() {
                refs.extend(explicit_workload_ref(source, &workload_ids));
                refs.extend(config_refs_for_root(source, &managed_configs));
            }
        } else if let Some(value) = directive_value(line, "Network") {
            refs.extend(explicit_workload_ref(value, &workload_ids));
        } else if let Some(value) = directive_value(line, "Sockets") {
            refs.extend(unit_refs_for_list(value, &workload_ids));
        } else if let Some(value) = directive_value(line, "After")
            .or_else(|| directive_value(line, "Requires"))
            .or_else(|| directive_value(line, "Wants"))
        {
            refs.extend(unit_refs_for_list(value, &workload_ids));
        }
    }

    refs.sort();
    refs.dedup();
    refs
}

fn directive_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(&format!("{key}="))
        .map(str::trim)
        .map(|value| value.trim_start_matches('-'))
}

fn config_refs_for_path(path: &str, managed_configs: &BTreeSet<&str>) -> Vec<String> {
    if path.starts_with('/') && managed_configs.contains(path) {
        vec![path.to_string()]
    } else {
        Vec::new()
    }
}

fn config_refs_for_root(root: &str, managed_configs: &BTreeSet<&str>) -> Vec<String> {
    if !root.starts_with('/') {
        return Vec::new();
    }
    managed_configs
        .iter()
        .filter(|path| **path == root || path.starts_with(&format!("{root}/")))
        .map(|path| (*path).to_string())
        .collect()
}

fn explicit_workload_ref(value: &str, workload_ids: &BTreeSet<&str>) -> Vec<String> {
    let direct = value.trim();
    let candidates = [
        direct.to_string(),
        format!("{direct}.network"),
        format!("{direct}.volume"),
        format!("{direct}.socket"),
        format!("{direct}.mount"),
        format!("{direct}.automount"),
        format!("{direct}.container"),
        format!("{direct}.service"),
    ];
    candidates
        .into_iter()
        .find(|candidate| workload_ids.contains(candidate.as_str()))
        .into_iter()
        .collect()
}

fn unit_refs_for_list(value: &str, workload_ids: &BTreeSet<&str>) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|token| workload_ids.contains(*token))
        .map(|token| token.to_string())
        .collect()
}

pub fn evaluate_desired_state(
    input: &EvaluationInput,
) -> Result<EvaluationOutput, EvaluationError> {
    let mut artifacts = Vec::new();
    let mut socket_dropins = Vec::new();
    for service_name in &input.host.services {
        let service =
            input.catalog.services.get(service_name).ok_or_else(|| {
                EvaluationError::new(format!("missing service: {}", service_name))
            })?;
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
        if !matches!(
            artifact.quadlet_type,
            QuadletType::Container | QuadletType::Pod
        ) {
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

fn desired_object_kind(quadlet_type: &QuadletType) -> ManagedObjectKind {
    match quadlet_type {
        QuadletType::Mount => ManagedObjectKind::Mount,
        QuadletType::Automount => ManagedObjectKind::Automount,
        QuadletType::ConfigFile => ManagedObjectKind::RenderedArtifact,
        _ => ManagedObjectKind::QuadletResource,
    }
}

fn collect_mount_declarations(
    _input: &EvaluationInput,
    artifacts: &[EvaluatedArtifact],
) -> Result<Vec<MountDeclaration>, EvaluationError> {
    let mut mounts = collect_mount_declarations_from_artifacts(artifacts)?;
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
        validate_x_coreops_keys(&section, &[], &artifact.name)?;
        if section.contains_key("X-CoreOps") {
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
        let mount_ids = mount_ids_for_service_artifacts(&service.artifacts, &declaration_map);
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

fn mount_ids_for_service_artifacts(
    artifacts: &[crate::core::types::ArtifactSource],
    declaration_map: &BTreeMap<&str, &MountDeclaration>,
) -> Vec<String> {
    let mut mount_ids = BTreeSet::new();
    let mount_units = declaration_map
        .values()
        .map(|decl| (decl.mount_unit_name(), decl.id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let automount_units = declaration_map
        .values()
        .filter_map(|decl| {
            decl.automount_unit_name()
                .map(|unit| (unit, decl.id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();

    for artifact in artifacts {
        if !matches!(
            artifact.quadlet_type,
            QuadletType::Container | QuadletType::Pod
        ) {
            continue;
        }
        for raw_line in artifact.contents.lines() {
            let line = raw_line.trim();
            if let Some(value) = directive_value(line, "RequiresMountsFor") {
                for consumed_path in value.split_whitespace() {
                    for declaration in declaration_map.values() {
                        if consumed_path == declaration.target_path
                            || consumed_path.starts_with(&format!(
                                "{}/",
                                declaration.target_path.trim_end_matches('/')
                            ))
                        {
                            mount_ids.insert(declaration.id.clone());
                        }
                    }
                }
            } else if let Some(value) =
                directive_value(line, "After").or_else(|| directive_value(line, "Requires"))
            {
                for unit in value.split_whitespace() {
                    if let Some(mount_id) =
                        mount_units.get(unit).or_else(|| automount_units.get(unit))
                    {
                        mount_ids.insert((*mount_id).to_string());
                    }
                }
            }
        }
    }

    mount_ids.into_iter().collect()
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
    prepared_path: Option<PreparedTargetPath>,
}

impl ParsedManagedMount {
    fn from_mount_artifact(artifact: &EvaluatedArtifact) -> Result<Option<Self>, EvaluationError> {
        let sections = parse_sections(&artifact.contents);
        if !sections.contains_key("X-CoreOps") {
            return Ok(None);
        }
        validate_x_coreops_keys(&sections, &["CreateMountpoint"], &artifact.name)?;
        let target_path = required_section_value(&sections, "Mount", "Where", &artifact.name)?;
        let expected_name = mount_unit_name_for_path(&target_path);
        if artifact.name != expected_name {
            return Err(EvaluationError::new(format!(
                "mount unit name does not match Mount Where in {}: expected {}",
                artifact.name, expected_name
            )));
        }
        let source = required_section_value(&sections, "Mount", "What", &artifact.name)?;
        let fstype = required_section_value(&sections, "Mount", "Type", &artifact.name)?;
        let mount_options = section_value(&sections, "Mount", "Options")
            .map(|value| split_csv(&value))
            .unwrap_or_default();
        let network_backed = section_bool_value(&sections, "X-CoreOps", "NetworkBacked")
            .unwrap_or_else(|| is_network_fstype(&fstype));
        let verification_mode = MountVerificationMode::UnitAndPath;
        let prepared_path = parse_prepared_path(&sections, &target_path);

        Ok(Some(Self {
            id: unit_stem(&artifact.name),
            target_path,
            source,
            fstype,
            mount_options,
            network_backed,
            verification_mode,
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
                validate_x_coreops_keys(&sections, &[], &artifact.name)?;
                let where_path =
                    required_section_value(&sections, "Automount", "Where", &artifact.name)?;
                if where_path != self.target_path {
                    return Err(EvaluationError::new(format!(
                        "automount Where does not match mount target: {} != {}",
                        where_path, self.target_path
                    )));
                }
                let expected_name = automount_unit_name_for_path(&where_path);
                if artifact.name != expected_name {
                    return Err(EvaluationError::new(format!(
                        "automount unit name does not match Automount Where in {}: expected {}",
                        artifact.name, expected_name
                    )));
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
    sections
        .get(section)
        .and_then(|values| values.get(key))
        .cloned()
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
) -> Option<PreparedTargetPath> {
    Some(PreparedTargetPath {
        path: target_path.to_string(),
        create_if_missing: section_bool_value(sections, "X-CoreOps", "CreateMountpoint")
            .unwrap_or(true),
    })
}

fn validate_x_coreops_keys(
    sections: &BTreeMap<String, BTreeMap<String, String>>,
    allowed: &[&str],
    unit_name: &str,
) -> Result<MountVerificationMode, EvaluationError> {
    let Some(values) = sections.get("X-CoreOps") else {
        return Ok(MountVerificationMode::UnitAndPath);
    };
    for key in values.keys() {
        if !allowed.iter().any(|allowed_key| key == allowed_key) {
            return Err(EvaluationError::new(format!(
                "unsupported X-CoreOps field in {}: {}",
                unit_name, key
            )));
        }
    }
    Ok(MountVerificationMode::UnitAndPath)
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
