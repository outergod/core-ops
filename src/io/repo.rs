use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::core::evaluate::{evaluate_desired_state, EvaluationOutput};
use crate::core::types::{
    ArtifactSource, Boundaries, BoundaryScope, ConfigFileSource, DesiredState, DropInSource,
    EnabledState, EvaluationInput, EvaluatedArtifact, EvaluatedConfigFile, EvaluatedDropIn,
    HostDeclaration, HostOverlaySet, Invariant, QuadletType, RestartPolicy, ServiceCatalog,
    ServiceDefinition, Workload,
};
use crate::io::quadlet::{parse_quadlet_name, read_quadlet_dir, QuadletError};
use crate::core::validation::{
    validate_config_paths, validate_dropin_targets as validate_dropin_targets_fn,
    validate_service_selection,
};
use serde::Deserialize;

pub const HOST_OVERRIDE_ENV: &str = "CORE_OPS_HOST";

#[derive(Debug)]
pub struct LayeredRepo {
    pub repo_path: PathBuf,
    pub revision_id: String,
    pub host: HostDeclaration,
    pub catalog: ServiceCatalog,
    pub overlays: HostOverlaySet,
}

#[derive(Debug)]
pub enum RepoError {
    GitCloneFailed(String),
    GitFetchFailed(String),
    GitCheckoutFailed(String),
    InvalidRepoSource(String),
    MissingQuadletDir(PathBuf),
    MissingServicesDir(PathBuf),
    MissingHostsDir(PathBuf),
    MissingHostDeclaration(PathBuf),
    InvalidHostDeclaration(String),
    MissingHostIdentity,
    Quadlet(QuadletError),
    Io(String),
    EvaluationFailed(String),
    ValidationFailed(String),
    InvalidDropIn(String),
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
            RepoError::MissingServicesDir(path) => {
                write!(f, "missing services dir: {}", path.display())
            }
            RepoError::MissingHostsDir(path) => {
                write!(f, "missing hosts dir: {}", path.display())
            }
            RepoError::MissingHostDeclaration(path) => {
                write!(f, "missing host declaration: {}", path.display())
            }
            RepoError::InvalidHostDeclaration(msg) => {
                write!(f, "invalid host declaration: {}", msg)
            }
            RepoError::MissingHostIdentity => write!(f, "missing host identity"),
            RepoError::Quadlet(err) => write!(f, "quadlet error: {}", err),
            RepoError::Io(err) => write!(f, "repo io error: {}", err),
            RepoError::EvaluationFailed(err) => write!(f, "evaluation failed: {}", err),
            RepoError::ValidationFailed(err) => write!(f, "validation failed: {}", err),
            RepoError::InvalidDropIn(err) => write!(f, "invalid drop-in: {}", err),
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
    let services_dir = repo_path.join("services");
    if services_dir.exists() {
        return load_layered_desired_state(&repo_path, revision_id);
    }
    let quadlet_dir = repo_path.join("quadlets");
    if !quadlet_dir.exists() {
        return Err(RepoError::MissingQuadletDir(quadlet_dir));
    }
    let workloads = read_quadlet_dir(&quadlet_dir)?;
    Ok(desired_state_from_workloads(
        &repo_path,
        revision_id,
        workloads,
        Vec::new(),
    ))
}

pub fn load_layered_repo(repo_source: &str, revision_id: &str) -> Result<LayeredRepo, RepoError> {
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
    let services_dir = repo_path.join("services");
    if !services_dir.exists() {
        return Err(RepoError::MissingServicesDir(services_dir));
    }
    let hosts_dir = repo_path.join("hosts");
    if !hosts_dir.exists() {
        return Err(RepoError::MissingHostsDir(hosts_dir));
    }

    let host_id = resolve_host_identity()?;
    let host_dir = hosts_dir.join(&host_id);
    let host_decl = load_host_declaration(&host_dir)?;
    let catalog = load_service_catalog(&services_dir)?;
    let mut overlays = load_host_overrides(&host_dir)?;
    let allowed_prefixes = config_prefixes_for_services(&host_decl.services);
    if let Some(err) = validate_config_overrides(&overlays.config_overrides, &allowed_prefixes) {
        return Err(RepoError::ValidationFailed(err));
    }
    overlays.config_overrides = filter_config_overrides(&overlays.config_overrides, &allowed_prefixes);

    Ok(LayeredRepo {
        repo_path,
        revision_id: revision_id.to_string(),
        host: host_decl,
        catalog,
        overlays,
    })
}

pub fn load_host_declaration(host_dir: &Path) -> Result<HostDeclaration, RepoError> {
    load_host_declaration_inner(host_dir)
}

fn load_layered_desired_state(
    repo_path: &Path,
    revision_id: &str,
) -> Result<DesiredState, RepoError> {
    let services_dir = repo_path.join("services");
    let hosts_dir = repo_path.join("hosts");
    if !hosts_dir.exists() {
        return Err(RepoError::MissingHostsDir(hosts_dir));
    }
    let host_id = resolve_host_identity()?;
    let host_dir = hosts_dir.join(&host_id);
    let host_decl = load_host_declaration_inner(&host_dir)?;
    let catalog = load_service_catalog(&services_dir)?;
    validate_service_selection(&host_decl, &catalog)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;
    let mut overlays = load_host_overrides(&host_dir)?;
    let allowed_prefixes = config_prefixes_for_services(&host_decl.services);
    if let Some(err) = validate_config_overrides(&overlays.config_overrides, &allowed_prefixes) {
        return Err(RepoError::ValidationFailed(err));
    }
    overlays.config_overrides = filter_config_overrides(&overlays.config_overrides, &allowed_prefixes);
    let all_artifacts = all_service_artifacts(&catalog);
    validate_dropin_targets(&catalog, &overlays, &all_artifacts)?;
    let mut config_paths = collect_config_paths(&host_decl.services, &catalog, &overlays);
    config_paths.sort();
    config_paths.dedup();
    validate_config_paths(&config_paths)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;

    let input = EvaluationInput {
        host: host_decl,
        catalog,
        overlays,
    };
    let output = evaluate_desired_state(&input)
        .map_err(|err| RepoError::EvaluationFailed(err.to_string()))?;
    let workloads = workloads_from_evaluation(&output);
    Ok(desired_state_from_workloads(
        repo_path,
        revision_id,
        workloads,
        config_paths,
    ))
}

pub fn desired_state_from_workloads(
    repo_path: &Path,
    revision_id: &str,
    workloads: Vec<Workload>,
    managed_config_paths: Vec<String>,
) -> DesiredState {
    DesiredState {
        repository_ref: repo_path.display().to_string(),
        revision_id: revision_id.to_string(),
        workloads,
        managed_config_paths,
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

fn resolve_host_identity() -> Result<String, RepoError> {
    if let Ok(value) = std::env::var(HOST_OVERRIDE_ENV) {
        if !value.trim().is_empty() {
            return Ok(value);
        }
    }
    let hostname = read_hostname().map_err(|err| RepoError::Io(err.to_string()))?;
    if hostname.trim().is_empty() {
        return Err(RepoError::MissingHostIdentity);
    }
    Ok(hostname)
}

fn load_host_declaration_inner(host_dir: &Path) -> Result<HostDeclaration, RepoError> {
    let host_yaml_path = host_dir.join("host.yaml");
    if !host_yaml_path.exists() {
        return Err(RepoError::MissingHostDeclaration(host_yaml_path));
    }
    let contents = fs::read_to_string(&host_yaml_path)
        .map_err(|err| RepoError::Io(err.to_string()))?;
    let parsed: HostYaml =
        serde_yaml::from_str(&contents).map_err(|err| RepoError::InvalidHostDeclaration(err.to_string()))?;
    let host_name = host_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RepoError::InvalidHostDeclaration("invalid host directory".to_string()))?;
    if parsed.host != host_name {
        return Err(RepoError::InvalidHostDeclaration(format!(
            "host field '{}' does not match directory '{}'",
            parsed.host, host_name
        )));
    }
    Ok(HostDeclaration {
        host: parsed.host,
        services: parsed.services,
    })
}

fn load_service_catalog(services_dir: &Path) -> Result<ServiceCatalog, RepoError> {
    let mut services = std::collections::BTreeMap::new();
    for entry in fs::read_dir(services_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let service_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name.to_string(),
            _ => continue,
        };
        let service = load_service_definition(&service_name, &path)?;
        services.insert(service_name, service);
    }

    Ok(ServiceCatalog { services })
}

fn load_service_definition(
    service_name: &str,
    service_dir: &Path,
) -> Result<ServiceDefinition, RepoError> {
    let mut artifacts = Vec::new();
    let mut base_dropins = Vec::new();
    let mut config_files = Vec::new();
    for entry in fs::read_dir(service_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name.to_string(),
            _ => continue,
        };
        if path.is_dir() {
            if file_name == "quadlet" {
                artifacts.extend(read_quadlet_files(&path)?);
                continue;
            }
            if file_name == "quadlet-overrides" {
                base_dropins.extend(read_dropins_from_root(&path)?);
                continue;
            }
            if let Some(target) = dropin_target_from_dir(&file_name) {
                base_dropins.extend(read_dropins(&path, &target)?);
                continue;
            }
            if file_name == "config" {
                config_files.extend(read_config_files(&path)?);
                continue;
            }
            continue;
        }
        if let Ok((_, quadlet_type)) = parse_quadlet_name(&file_name) {
            let contents =
                fs::read_to_string(&path).map_err(|err| RepoError::Io(err.to_string()))?;
            artifacts.push(ArtifactSource {
                name: file_name,
                quadlet_type,
                contents,
                source_path: path.display().to_string(),
            });
        }
    }

    Ok(ServiceDefinition {
        name: service_name.to_string(),
        artifacts,
        base_dropins,
        config_files,
    })
}

fn load_host_overrides(host_dir: &Path) -> Result<HostOverlaySet, RepoError> {
    let overrides_dir = host_dir.join("overrides");
    let host_name = host_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RepoError::InvalidHostDeclaration("invalid host directory".to_string()))?;
    if !overrides_dir.exists() {
        return Ok(HostOverlaySet {
            host: host_name.to_string(),
            overrides: Vec::new(),
            config_overrides: Vec::new(),
        });
    }

    let mut overrides = Vec::new();
    let mut config_overrides = Vec::new();
    for entry in fs::read_dir(&overrides_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name.to_string(),
            _ => continue,
        };
        if file_name == "quadlet" {
            overrides.extend(read_dropins_from_root(&path)?);
            continue;
        }
        if let Some(target) = dropin_target_from_dir(&file_name) {
            overrides.extend(read_dropins(&path, &target)?);
            continue;
        }
        if file_name == "config" {
            config_overrides.extend(read_config_files(&path)?);
        }
    }

    Ok(HostOverlaySet {
        host: host_name.to_string(),
        overrides,
        config_overrides,
    })
}

fn dropin_target_from_dir(dir_name: &str) -> Option<String> {
    dir_name.strip_suffix(".d").map(|name| name.to_string())
}

fn read_dropins(dir: &Path, target: &str) -> Result<Vec<DropInSource>, RepoError> {
    let mut dropins = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name,
            _ => continue,
        };
        if !file_name.ends_with(".conf") {
            return Err(RepoError::InvalidDropIn(format!(
                "unsupported drop-in extension: {}",
                file_name
            )));
        }
        let contents =
            fs::read_to_string(&path).map_err(|err| RepoError::Io(err.to_string()))?;
        dropins.push(DropInSource {
            target: target.to_string(),
            contents,
            source_path: path.display().to_string(),
        });
    }
    Ok(dropins)
}

fn read_dropins_from_root(root: &Path) -> Result<Vec<DropInSource>, RepoError> {
    let mut dropins = Vec::new();
    if !root.exists() {
        return Ok(dropins);
    }
    for entry in fs::read_dir(root).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name.to_string(),
            _ => continue,
        };
        if let Some(target) = dropin_target_from_dir(&file_name) {
            dropins.extend(read_dropins(&path, &target)?);
        }
    }
    Ok(dropins)
}

fn read_hostname() -> Result<String, std::io::Error> {
    let mut buf = [0u8; 256];
    let result = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if result != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let len = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    Ok(String::from_utf8_lossy(&buf[..len]).trim().to_string())
}

fn workloads_from_evaluation(output: &EvaluationOutput) -> Vec<Workload> {
    let mut workloads: Vec<Workload> = output
        .artifacts
        .iter()
        .map(workload_from_artifact)
        .collect();
    workloads.extend(
        output
            .socket_dropins
            .iter()
            .map(workload_from_socket_dropin),
    );
    workloads.extend(
        output
            .config_files
            .iter()
            .map(workload_from_config_file),
    );
    workloads
}

fn workload_from_artifact(artifact: &EvaluatedArtifact) -> Workload {
    let contents = if artifact.quadlet_type == QuadletType::Socket {
        crate::io::quadlet::normalize_socket_contents(&artifact.contents)
    } else {
        artifact.contents.clone()
    };
    let name = Path::new(&artifact.name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(&artifact.name)
        .to_string();
    Workload {
        name,
        quadlet_type: artifact.quadlet_type.clone(),
        quadlet_contents: contents,
        systemd_unit_name: artifact.name.clone(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn workload_from_socket_dropin(dropin: &EvaluatedDropIn) -> Workload {
    Workload {
        name: format!("{}.d/{}", dropin.target, dropin.file_name),
        quadlet_type: QuadletType::SocketDropIn,
        quadlet_contents: dropin.contents.clone(),
        systemd_unit_name: format!("{}.d/{}", dropin.target, dropin.file_name),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn workload_from_config_file(file: &EvaluatedConfigFile) -> Workload {
    Workload {
        name: file.target_path.clone(),
        quadlet_type: QuadletType::ConfigFile,
        quadlet_contents: file.contents.clone(),
        systemd_unit_name: file.target_path.clone(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn all_service_artifacts(catalog: &ServiceCatalog) -> Vec<ArtifactSource> {
    let mut artifacts = Vec::new();
    for service in catalog.services.values() {
        artifacts.extend(service.artifacts.iter().cloned());
    }
    artifacts
}

fn validate_dropin_targets(
    catalog: &ServiceCatalog,
    overlays: &HostOverlaySet,
    artifacts: &[ArtifactSource],
) -> Result<(), RepoError> {
    let mut dropins = Vec::new();
    for service in catalog.services.values() {
        dropins.extend(service.base_dropins.iter().cloned());
    }
    dropins.extend(overlays.overrides.iter().cloned());
    validate_dropin_targets_fn(&dropins, artifacts)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))
}

fn read_config_files(config_root: &Path) -> Result<Vec<ConfigFileSource>, RepoError> {
    let mut files = Vec::new();
    for entry in walk_config_dir(config_root)? {
        let rel = entry.strip_prefix(config_root).map_err(|err| RepoError::Io(err.to_string()))?;
        let rel_str = rel.to_string_lossy();
        if rel_str.starts_with("etc/") {
            let contents = fs::read_to_string(&entry).map_err(|err| RepoError::Io(err.to_string()))?;
            let target_path = format!("/{}", rel_str);
            files.push(ConfigFileSource {
                target_path,
                contents,
                source_path: entry.display().to_string(),
            });
        }
    }
    Ok(files)
}

fn walk_config_dir(root: &Path) -> Result<Vec<PathBuf>, RepoError> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_config_dir(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

fn collect_config_paths(
    selected_services: &[String],
    catalog: &ServiceCatalog,
    overlays: &HostOverlaySet,
) -> Vec<String> {
    let mut paths = Vec::new();
    for service_name in selected_services {
        if let Some(service) = catalog.services.get(service_name) {
            paths.extend(service.config_files.iter().map(|f| f.target_path.clone()));
        }
    }
    paths.extend(
        overlays
            .config_overrides
            .iter()
            .map(|f| f.target_path.clone()),
    );
    paths
}

fn config_prefixes_for_services(services: &[String]) -> Vec<String> {
    services
        .iter()
        .map(|service| format!("/etc/{service}/"))
        .collect()
}

fn validate_config_overrides(
    overrides: &[ConfigFileSource],
    allowed_prefixes: &[String],
) -> Option<String> {
    let invalid: Vec<&ConfigFileSource> = overrides
        .iter()
        .filter(|cfg| !allowed_prefixes.iter().any(|prefix| cfg.target_path.starts_with(prefix)))
        .collect();
    if invalid.is_empty() {
        return None;
    }
    Some(format!(
        "host config override outside selected services: {}",
        invalid
            .iter()
            .map(|cfg| cfg.target_path.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn filter_config_overrides(
    overrides: &[ConfigFileSource],
    allowed_prefixes: &[String],
) -> Vec<ConfigFileSource> {
    overrides
        .iter()
        .filter(|cfg| allowed_prefixes.iter().any(|prefix| cfg.target_path.starts_with(prefix)))
        .cloned()
        .collect()
}

fn read_quadlet_files(dir: &Path) -> Result<Vec<ArtifactSource>, RepoError> {
    let mut artifacts = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) if !name.starts_with('.') => name.to_string(),
            _ => continue,
        };
        if let Ok((_, quadlet_type)) = parse_quadlet_name(&file_name) {
            let contents =
                fs::read_to_string(&path).map_err(|err| RepoError::Io(err.to_string()))?;
            artifacts.push(ArtifactSource {
                name: file_name,
                quadlet_type,
                contents,
                source_path: path.display().to_string(),
            });
        }
    }
    Ok(artifacts)
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

#[derive(Debug, Deserialize)]
struct HostYaml {
    host: String,
    services: Vec<String>,
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
