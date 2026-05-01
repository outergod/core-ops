use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::core::evaluate::{evaluate_desired_state, EvaluationOutput};
use crate::core::types::{
    ArtifactSource, Boundaries, BoundaryScope, ConfigFileSource, DesiredState, DropInSource,
    EnabledState, EvaluatedArtifact, EvaluatedConfigFile, EvaluatedDropIn, EvaluationInput,
    HostDeclaration, HostOverlaySet, Invariant, MountDeclaration, QuadletType, RestartPolicy,
    ServiceCatalog, ServiceDefinition, Workload,
};
use crate::core::validation::{
    validate_config_paths, validate_dropin_targets as validate_dropin_targets_fn,
    validate_mount_model, validate_service_selection,
};
use crate::io::quadlet::{parse_quadlet_name, QuadletError};
use serde::Deserialize;

pub const HOST_OVERRIDE_ENV: &str = "CORE_OPS_HOST";
pub const NATIVE_UNIT_MANAGED_MARKER: &str = "# Managed by CoreOps";

#[derive(Debug)]
pub struct LayeredRepo {
    _repo_temp: TempDir,
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
    MissingServicesDir(PathBuf),
    MissingHostsDir(PathBuf),
    MissingHostDeclaration(PathBuf),
    InvalidHostDeclaration(String),
    InvalidServiceManifest(String),
    MissingHostIdentity,
    Quadlet(QuadletError),
    Io(String),
    EvaluationFailed(String),
    ValidationFailed(String),
    InvalidDropIn(String),
    /// Source repository contains a legacy-layout artifact that the
    /// formalized loader (spec 016) refuses to process. The migration
    /// script `scripts/migrate-legacy-source-repo.sh` produces a
    /// formalized-layout copy in one mechanical pass.
    LegacyArtifact(PathBuf),
    /// Service id, host id, or payload-kind name begins with `_` or `.`,
    /// which is reserved for future metadata.
    ReservedName(String),
    /// Identifier (service id, host id, or `config-root`) does not match
    /// the documented pattern `[A-Za-z0-9][A-Za-z0-9._-]*`. Without this
    /// check, values containing `/` could escape `/etc/<config-root>/`
    /// at observed-state scan time and drive unintended removals.
    InvalidIdentifier(String),
    /// Host overlay attempted to introduce a base unit; only drop-ins
    /// (`<unit>.<ext>.d/<file>.conf`) and `config/` whole-file
    /// replacements are permitted.
    HostOverlayBaseUnit(PathBuf),
    /// Payload-kind directory contains a file whose extension is not
    /// recognized for that kind (e.g. `quadlet/foo.socket` or
    /// `systemd/foo.container`).
    InvalidPayloadKindFile { path: PathBuf, kind: &'static str },
    /// `config/` file destination escapes `/etc/<config-root>/` (e.g.
    /// via `..` segments) — see FR-010.
    ConfigEscape { source_path: PathBuf, config_root: String },
    /// A drop-in references a parent unit that does not exist in the
    /// merged service+host overlay set — see FR-013.
    OrphanDropIn { path: PathBuf, unit: String },
    /// Two distinct source files compute to the same host destination
    /// path — see FR-011.
    DestinationConflict { target: PathBuf, a: PathBuf, b: PathBuf },
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
            RepoError::InvalidServiceManifest(msg) => {
                write!(f, "invalid service manifest: {}", msg)
            }
            RepoError::MissingHostIdentity => write!(f, "missing host identity"),
            RepoError::Quadlet(err) => write!(f, "quadlet error: {}", err),
            RepoError::Io(err) => write!(f, "repo io error: {}", err),
            RepoError::EvaluationFailed(err) => write!(f, "evaluation failed: {}", err),
            RepoError::ValidationFailed(err) => write!(f, "validation failed: {}", err),
            RepoError::InvalidDropIn(err) => write!(f, "invalid drop-in: {}", err),
            RepoError::LegacyArtifact(path) => write!(
                f,
                "legacy layout artifact: {} (run scripts/migrate-legacy-source-repo.sh)",
                path.display()
            ),
            RepoError::ReservedName(name) => write!(
                f,
                "reserved name '{}' (must not begin with '_' or '.')",
                name
            ),
            RepoError::InvalidIdentifier(name) => write!(
                f,
                "invalid identifier '{}' (must match [A-Za-z0-9][A-Za-z0-9._-]*)",
                name
            ),
            RepoError::HostOverlayBaseUnit(path) => write!(
                f,
                "host overlay introduces base unit at {} (only drop-ins and config replacements allowed)",
                path.display()
            ),
            RepoError::InvalidPayloadKindFile { path, kind } => write!(
                f,
                "{} payload kind cannot accept file: {}",
                kind,
                path.display()
            ),
            RepoError::ConfigEscape { source_path, config_root } => write!(
                f,
                "config file destination escapes /etc/{}/: {}",
                config_root,
                source_path.display()
            ),
            RepoError::OrphanDropIn { path, unit } => write!(
                f,
                "orphan drop-in at {} (no matching unit '{}' in merged set)",
                path.display(),
                unit
            ),
            RepoError::DestinationConflict { target, a, b } => write!(
                f,
                "destination conflict at {}: {} and {}",
                target.display(),
                a.display(),
                b.display()
            ),
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
    git_checkout_revision(temp.path(), revision_id)?;

    let repo_path = temp.path().to_path_buf();
    validate_no_legacy_root_artifacts(&repo_path)?;
    let services_dir = repo_path.join("services");
    if !services_dir.exists() {
        return Err(RepoError::MissingServicesDir(services_dir));
    }
    load_layered_desired_state(&repo_path, repo_source, revision_id)
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
    git_checkout_revision(temp.path(), revision_id)?;

    let repo_path = temp.path().to_path_buf();
    validate_no_legacy_root_artifacts(&repo_path)?;
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
    validate_service_selection(&host_decl, &catalog)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;
    let mut overlays = load_host_overrides(&host_dir, &catalog, &host_decl.services)?;
    let allowed_prefixes = config_prefixes_for_services(&host_decl.services, &catalog);
    if let Some(err) = validate_config_overrides(&overlays.config_overrides, &allowed_prefixes) {
        return Err(RepoError::ValidationFailed(err));
    }
    overlays.config_overrides =
        filter_config_overrides(&overlays.config_overrides, &allowed_prefixes);
    validate_config_destination_conflicts(&host_decl.services, &catalog, &overlays)?;
    let all_artifacts = selected_service_artifacts(&host_decl.services, &catalog);
    validate_dropin_targets(&host_decl.services, &catalog, &overlays, &all_artifacts)?;

    Ok(LayeredRepo {
        _repo_temp: temp,
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
    requested_repository: &str,
    requested_ref: &str,
) -> Result<DesiredState, RepoError> {
    let services_dir = repo_path.join("services");
    let hosts_dir = repo_path.join("hosts");
    if !hosts_dir.exists() {
        return Err(RepoError::MissingHostsDir(hosts_dir));
    }
    validate_no_legacy_root_artifacts(repo_path)?;
    let host_id = resolve_host_identity()?;
    let host_dir = hosts_dir.join(&host_id);
    let host_decl = load_host_declaration_inner(&host_dir)?;
    let catalog = load_service_catalog(&services_dir)?;
    validate_service_selection(&host_decl, &catalog)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;
    let mut overlays = load_host_overrides(&host_dir, &catalog, &host_decl.services)?;
    let allowed_prefixes = config_prefixes_for_services(&host_decl.services, &catalog);
    if let Some(err) = validate_config_overrides(&overlays.config_overrides, &allowed_prefixes) {
        return Err(RepoError::ValidationFailed(err));
    }
    overlays.config_overrides =
        filter_config_overrides(&overlays.config_overrides, &allowed_prefixes);
    validate_config_destination_conflicts(&host_decl.services, &catalog, &overlays)?;
    let all_artifacts = selected_service_artifacts(&host_decl.services, &catalog);
    validate_dropin_targets(&host_decl.services, &catalog, &overlays, &all_artifacts)?;
    let mut config_paths = collect_config_paths(&host_decl.services, &catalog, &overlays);
    config_paths.sort();
    config_paths.dedup();
    let mut config_roots = config_roots_for_paths(&config_paths);
    config_roots.sort();
    config_roots.dedup();
    validate_config_paths(&config_paths)
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;

    let input = EvaluationInput {
        host: host_decl,
        catalog,
        overlays,
    };
    let output = evaluate_desired_state(&input)
        .map_err(|err| RepoError::EvaluationFailed(err.to_string()))?;
    validate_mount_model(
        &output.mount_declarations,
        &output.mount_dependencies,
        Some(&input.host.services),
    )
    .map_err(|err| RepoError::ValidationFailed(err.to_string()))?;
    let workloads = workloads_from_evaluation(&output);
    let resolved_revision = resolved_head_revision(repo_path)?;
    Ok(desired_state_from_workloads(
        repo_path,
        DesiredStateInputs {
            revision_id: &resolved_revision,
            requested_repository: Some(requested_repository.to_string()),
            requested_ref: Some(requested_ref.to_string()),
            workloads,
            mount_declarations: output.mount_declarations,
            mount_dependencies: output.mount_dependencies,
            managed_config_paths: config_paths,
            managed_config_roots: config_roots,
        },
    ))
}

pub struct DesiredStateInputs<'a> {
    pub revision_id: &'a str,
    pub requested_repository: Option<String>,
    pub requested_ref: Option<String>,
    workloads: Vec<Workload>,
    pub mount_declarations: Vec<MountDeclaration>,
    pub mount_dependencies: Vec<crate::core::types::MountDependency>,
    pub managed_config_paths: Vec<String>,
    pub managed_config_roots: Vec<String>,
}

pub fn desired_state_from_workloads(
    repo_path: &Path,
    inputs: DesiredStateInputs<'_>,
) -> DesiredState {
    DesiredState {
        repository_ref: repo_path.display().to_string(),
        revision_id: inputs.revision_id.to_string(),
        requested_repository: inputs.requested_repository,
        requested_ref: inputs.requested_ref,
        workloads: inputs.workloads,
        mount_declarations: inputs.mount_declarations,
        mount_dependencies: inputs.mount_dependencies,
        managed_config_paths: inputs.managed_config_paths,
        managed_config_roots: inputs.managed_config_roots,
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
    let contents =
        fs::read_to_string(&host_yaml_path).map_err(|err| RepoError::Io(err.to_string()))?;
    let parsed: HostYaml = serde_yaml::from_str(&contents)
        .map_err(|err| RepoError::InvalidHostDeclaration(err.to_string()))?;
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
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        // Hidden entries (starting with `.`) are tolerated — they're not
        // service identifiers, just opaque metadata (e.g., `.gitkeep`).
        // Reserved-prefix entries (`_*`) are rejected per FR-009.
        if file_name.starts_with('.') {
            continue;
        }
        if !path.is_dir() {
            // A file directly under `services/` is unexpected and likely a
            // misplaced artifact. Reject loudly.
            return Err(RepoError::LegacyArtifact(path));
        }
        validate_id(&file_name)?;
        let service = load_service_definition(&file_name, &path)?;
        services.insert(file_name, service);
    }

    Ok(ServiceCatalog { services })
}

fn load_service_definition(
    service_name: &str,
    service_dir: &Path,
) -> Result<ServiceDefinition, RepoError> {
    // Resolve config-root from optional service.yaml (FR-006, FR-007).
    let manifest = load_service_manifest(service_dir)?;
    let config_root = manifest
        .as_ref()
        .map(|m| m.config_root.clone())
        .unwrap_or_else(|| service_name.to_string());
    validate_id(&config_root)?;

    let mut artifacts = Vec::new();
    let mut base_dropins = Vec::new();
    let mut config_files = Vec::new();

    // services/<svc>/ may contain ONLY: service.yaml (file), and the
    // payload-kind directories quadlet/, systemd/, config/. Any other
    // entry is either a legacy artifact or unrecognized noise — both
    // produce a load-time error so the operator gets a clear pointer.
    for entry in fs::read_dir(service_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Tolerate hidden metadata (.gitkeep, .DS_Store, etc.). Reserved
        // names beginning with `_` are forbidden as content directories
        // (FR-009) but tolerated as opaque files (e.g., `_local`).
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_file() {
            if file_name == "service.yaml" {
                continue; // already parsed
            }
            // Files at service root are legacy (a *.container, *.socket,
            // etc., directly under services/<svc>/). Reject loudly.
            return Err(RepoError::LegacyArtifact(path));
        }

        // Directory entries.
        match file_name.as_str() {
            "quadlet" => {
                artifacts.extend(read_payload_units(&path, PayloadKind::Quadlet)?);
                base_dropins.extend(read_payload_dropins(&path, PayloadKind::Quadlet)?);
            }
            "systemd" => {
                artifacts.extend(read_payload_units(&path, PayloadKind::Systemd)?);
                base_dropins.extend(read_payload_dropins(&path, PayloadKind::Systemd)?);
            }
            "config" => {
                config_files.extend(read_config_files(&path, &config_root)?);
            }
            "quadlet-overrides" => {
                // Legacy split-drop-ins directory.
                return Err(RepoError::LegacyArtifact(path));
            }
            other if other.ends_with(".d") => {
                // Legacy: drop-in directory at service root (instead of
                // nested inside quadlet/ or systemd/).
                return Err(RepoError::LegacyArtifact(path));
            }
            _ => {
                if file_name.starts_with('_') {
                    return Err(RepoError::ReservedName(file_name));
                }
                return Err(RepoError::LegacyArtifact(path));
            }
        }
    }

    Ok(ServiceDefinition {
        name: service_name.to_string(),
        config_root,
        artifacts,
        base_dropins,
        config_files,
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ServiceManifest {
    config_root: String,
}

fn load_service_manifest(service_dir: &Path) -> Result<Option<ServiceManifest>, RepoError> {
    let manifest_path = service_dir.join("service.yaml");
    if !manifest_path.exists() {
        return Ok(None);
    }
    let contents =
        fs::read_to_string(&manifest_path).map_err(|err| RepoError::Io(err.to_string()))?;
    let parsed: ServiceManifest = serde_yaml::from_str(&contents).map_err(|err| {
        RepoError::InvalidServiceManifest(format!(
            "{}: {}",
            manifest_path.display(),
            err
        ))
    })?;
    if parsed.config_root.is_empty() {
        return Err(RepoError::InvalidServiceManifest(format!(
            "{}: config-root is empty",
            manifest_path.display()
        )));
    }
    Ok(Some(parsed))
}

fn load_host_overrides(
    host_dir: &Path,
    catalog: &ServiceCatalog,
    selected_services: &[String],
) -> Result<HostOverlaySet, RepoError> {
    let host_name = host_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| RepoError::InvalidHostDeclaration("invalid host directory".to_string()))?;

    // Reject legacy `overrides/` subdirectory (FR-012).
    let legacy = host_dir.join("overrides");
    if legacy.exists() {
        return Err(RepoError::LegacyArtifact(legacy));
    }

    let mut overrides: Vec<DropInSource> = Vec::new();
    let mut config_overrides: Vec<ConfigFileSource> = Vec::new();

    for entry in fs::read_dir(host_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_file() {
            if file_name == "host.yaml" {
                continue;
            }
            return Err(RepoError::LegacyArtifact(path));
        }
        // Directory: per-service overlay.
        if file_name.starts_with('_') {
            return Err(RepoError::ReservedName(file_name));
        }
        let svc_id = &file_name;
        // The overlay directory's name MUST be a service this host
        // selects. A typo like `hosts/<h>/traefic-dnschallenge/` (note
        // the missing 'k') would otherwise inject drop-ins keyed by raw
        // unit name, with the parser falling back to using the
        // typo-name as the config-root and validate_dropin_targets only
        // checking unit-name existence — silent cross-service drift.
        // validate_service_selection already guarantees every entry in
        // `selected_services` exists in the catalog, so checking
        // membership here covers both "unknown service" and "known but
        // unselected" in a single shot.
        if !selected_services.iter().any(|s| s == svc_id) {
            return Err(RepoError::ValidationFailed(format!(
                "host '{host_name}' has overlay directory '{svc_id}' but \
                 host.yaml does not select that service; did you typo the \
                 directory name? (expected one of {selected_services:?})"
            )));
        }
        // Service is guaranteed in the catalog because
        // validate_service_selection covers the host.yaml selection.
        let config_root = catalog
            .services
            .get(svc_id)
            .map(|s| s.config_root.clone())
            .unwrap_or_else(|| svc_id.clone());

        let (svc_dropins, svc_configs) = walk_host_service_overlay(&path, &config_root)?;
        overrides.extend(svc_dropins);
        config_overrides.extend(svc_configs);
    }

    Ok(HostOverlaySet {
        host: host_name.to_string(),
        overrides,
        config_overrides,
    })
}

fn walk_host_service_overlay(
    overlay_dir: &Path,
    config_root: &str,
) -> Result<(Vec<DropInSource>, Vec<ConfigFileSource>), RepoError> {
    let mut dropins = Vec::new();
    let mut configs = Vec::new();
    for entry in fs::read_dir(overlay_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_file() {
            // A regular file under the overlay root would be a base
            // unit redefinition — disallowed.
            return Err(RepoError::HostOverlayBaseUnit(path));
        }
        match file_name.as_str() {
            kind_name @ ("quadlet" | "systemd") => {
                let kind = if kind_name == "quadlet" {
                    PayloadKind::Quadlet
                } else {
                    PayloadKind::Systemd
                };
                for child in fs::read_dir(&path).map_err(|err| RepoError::Io(err.to_string()))? {
                    let child = child.map_err(|err| RepoError::Io(err.to_string()))?;
                    let cpath = child.path();
                    let cname = match cpath.file_name().and_then(|name| name.to_str()) {
                        Some(name) => name.to_string(),
                        None => continue,
                    };
                    if cname.starts_with('.') {
                        continue;
                    }
                    if cpath.is_file() {
                        // Base unit in host overlay → reject (FR-018).
                        return Err(RepoError::HostOverlayBaseUnit(cpath));
                    }
                    if let Some(target) = dropin_target_from_dir(&cname) {
                        // Cross-kind drop-in check: a `*.container.d/`
                        // under a `systemd/` overlay (or `*.socket.d/`
                        // under `quadlet/`) is a typo, not legitimate
                        // configuration. Reject. Codex P2 on PR #28.
                        let (_, target_quadlet_type) = parse_quadlet_name(&target).map_err(
                            |_| RepoError::InvalidPayloadKindFile {
                                path: cpath.clone(),
                                kind: kind.name(),
                            },
                        )?;
                        if !kind.accepts(&target_quadlet_type) {
                            return Err(RepoError::InvalidPayloadKindFile {
                                path: cpath,
                                kind: kind.name(),
                            });
                        }
                        dropins.extend(read_dropins(&cpath, &target)?);
                    } else {
                        return Err(RepoError::LegacyArtifact(cpath));
                    }
                }
            }
            "config" => {
                configs.extend(read_config_files(&path, config_root)?);
            }
            _ => {
                if file_name.starts_with('_') {
                    return Err(RepoError::ReservedName(file_name));
                }
                return Err(RepoError::LegacyArtifact(path));
            }
        }
    }
    Ok((dropins, configs))
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
        let contents = fs::read_to_string(&path).map_err(|err| RepoError::Io(err.to_string()))?;
        dropins.push(DropInSource {
            target: target.to_string(),
            contents,
            source_path: path.display().to_string(),
        });
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
    let existing_native_units: std::collections::BTreeSet<String> = workloads
        .iter()
        .filter(|workload| {
            matches!(
                workload.quadlet_type,
                QuadletType::Mount | QuadletType::Automount
            )
        })
        .map(|workload| workload.systemd_unit_name.clone())
        .collect();
    workloads.extend(output.mount_declarations.iter().flat_map(|mount| {
        let mut units = Vec::new();
        if !existing_native_units.contains(&mount.mount_unit_name()) {
            units.push(workload_from_native_unit(
                &mount.mount_unit_name(),
                &render_generated_mount_unit(mount),
            ));
        }
        if let Some(automount_name) = mount.automount_unit_name() {
            if !existing_native_units.contains(&automount_name) {
                units.push(workload_from_native_unit(
                    &automount_name,
                    &render_generated_automount_unit(mount),
                ));
            }
        }
        units
    }));
    workloads.extend(
        output
            .socket_dropins
            .iter()
            .map(workload_from_socket_dropin),
    );
    workloads.extend(output.config_files.iter().map(workload_from_config_file));
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

fn workload_from_native_unit(unit_name: &str, contents: &str) -> Workload {
    let quadlet_type = if unit_name.ends_with(".automount") {
        QuadletType::Automount
    } else {
        QuadletType::Mount
    };
    let name = Path::new(unit_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(unit_name)
        .to_string();
    Workload {
        name,
        quadlet_type,
        quadlet_contents: contents.to_string(),
        systemd_unit_name: unit_name.to_string(),
        enabled_state: EnabledState::Enabled,
        restart_policy: RestartPolicy::Always,
    }
}

fn render_generated_mount_unit(mount: &MountDeclaration) -> String {
    let mut contents = format!(
        "{NATIVE_UNIT_MANAGED_MARKER}\n[Mount]\nWhat={}\nWhere={}\nType={}\n",
        mount.source, mount.target_path, mount.fstype
    );
    if !mount.mount_options.is_empty() {
        contents.push_str(&format!("Options={}\n", mount.mount_options.join(",")));
    }
    contents
}

fn render_generated_automount_unit(mount: &MountDeclaration) -> String {
    format!(
        "{NATIVE_UNIT_MANAGED_MARKER}\n[Automount]\nWhere={}\n",
        mount.target_path
    )
}

fn selected_service_artifacts(
    selected_services: &[String],
    catalog: &ServiceCatalog,
) -> Vec<ArtifactSource> {
    let mut artifacts = Vec::new();
    for service_name in selected_services {
        if let Some(service) = catalog.services.get(service_name) {
            artifacts.extend(service.artifacts.iter().cloned());
        }
    }
    artifacts
}

fn validate_dropin_targets(
    selected_services: &[String],
    catalog: &ServiceCatalog,
    overlays: &HostOverlaySet,
    artifacts: &[ArtifactSource],
) -> Result<(), RepoError> {
    let mut dropins = Vec::new();
    let mut base_dropins = Vec::new();
    for service_name in selected_services {
        if let Some(service) = catalog.services.get(service_name) {
            base_dropins.extend(service.base_dropins.iter().cloned());
        }
    }
    dropins.extend(base_dropins.iter().cloned());
    dropins.extend(overlays.overrides.iter().cloned());
    validate_dropin_targets_fn(&dropins, artifacts)
        .and_then(|_| {
            crate::core::validation::validate_socket_dropin_precedence(
                &base_dropins,
                &overlays.overrides,
            )
        })
        .map_err(|err| RepoError::ValidationFailed(err.to_string()))
}

fn read_config_files(
    config_dir: &Path,
    config_root: &str,
) -> Result<Vec<ConfigFileSource>, RepoError> {
    let mut files = Vec::new();
    if !config_dir.exists() {
        return Ok(files);
    }
    // FR-002: `config/<rel>` is generic — a literal subdir named `etc`
    // (e.g. `config/etc/foo` deploying to `/etc/<config-root>/etc/foo`)
    // is legitimate. The legacy `config/etc/<config-root>/<rel>` mirror
    // is detected at migration time by `scripts/migrate-legacy-source-
    // repo.sh` (which flattens it) and at load time by other unambiguous
    // markers (top-level `quadlets/`, `services/<svc>/quadlet-overrides/`,
    // `hosts/<h>/overrides/`); the parser does NOT special-case `etc/`
    // here.
    for entry in walk_config_dir(config_dir)? {
        let rel = entry
            .strip_prefix(config_dir)
            .map_err(|err| RepoError::Io(err.to_string()))?;
        // Reject path-traversal segments (FR-010). Filesystem walks should
        // never produce `..` components, but defend in depth.
        if rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(RepoError::ConfigEscape {
                source_path: entry.clone(),
                config_root: config_root.to_string(),
            });
        }
        let rel_str = rel.to_string_lossy();
        let contents =
            fs::read_to_string(&entry).map_err(|err| RepoError::Io(err.to_string()))?;
        let target_path = format!("/etc/{}/{}", config_root, rel_str);
        files.push(ConfigFileSource {
            target_path,
            contents,
            source_path: entry.display().to_string(),
        });
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

fn config_prefixes_for_services(
    selected_services: &[String],
    catalog: &ServiceCatalog,
) -> Vec<String> {
    let paths: Vec<String> = selected_services
        .iter()
        .filter_map(|service_name| catalog.services.get(service_name))
        .flat_map(|service| service.config_files.iter().map(|f| f.target_path.clone()))
        .collect();
    config_roots_for_paths(&paths)
        .into_iter()
        .map(|root| format!("{root}/"))
        .collect()
}

fn config_roots_for_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| managed_config_root(path))
        .collect()
}

fn managed_config_root(path: &str) -> Option<String> {
    let trimmed = path.strip_prefix("/etc/")?;
    let first_component = trimmed.split('/').next()?;
    if first_component.is_empty() {
        return None;
    }
    Some(format!("/etc/{first_component}"))
}

fn validate_config_overrides(
    overrides: &[ConfigFileSource],
    allowed_prefixes: &[String],
) -> Option<String> {
    let invalid: Vec<&ConfigFileSource> = overrides
        .iter()
        .filter(|cfg| {
            !allowed_prefixes
                .iter()
                .any(|prefix| cfg.target_path.starts_with(prefix))
        })
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

/// FR-011: reject any source repository in which two distinct files
/// compute to the same final destination path. Scans `config/` files
/// across all selected services for collisions, then scans the host
/// overlay set for collisions among override entries. Host overrides
/// intentionally win over base files at the same target — that's
/// override semantics, not a conflict — so we do NOT cross-check base
/// vs. overlay. Quadlet / native unit name collisions across services
/// are caught downstream by `validate_workloads`'s `DuplicateUnitName`.
fn validate_config_destination_conflicts(
    selected_services: &[String],
    catalog: &ServiceCatalog,
    overlays: &HostOverlaySet,
) -> Result<(), RepoError> {
    let mut by_target: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();
    for svc_id in selected_services {
        let Some(svc) = catalog.services.get(svc_id) else {
            continue;
        };
        for cf in &svc.config_files {
            let source = PathBuf::from(&cf.source_path);
            if let Some(existing) = by_target.insert(cf.target_path.clone(), source.clone()) {
                return Err(RepoError::DestinationConflict {
                    target: PathBuf::from(&cf.target_path),
                    a: existing,
                    b: source,
                });
            }
        }
    }
    let mut overlay_by_target: std::collections::BTreeMap<String, PathBuf> =
        std::collections::BTreeMap::new();
    for cf in &overlays.config_overrides {
        let source = PathBuf::from(&cf.source_path);
        if let Some(existing) = overlay_by_target.insert(cf.target_path.clone(), source.clone()) {
            return Err(RepoError::DestinationConflict {
                target: PathBuf::from(&cf.target_path),
                a: existing,
                b: source,
            });
        }
    }
    Ok(())
}

fn filter_config_overrides(
    overrides: &[ConfigFileSource],
    allowed_prefixes: &[String],
) -> Vec<ConfigFileSource> {
    overrides
        .iter()
        .filter(|cfg| {
            allowed_prefixes
                .iter()
                .any(|prefix| cfg.target_path.starts_with(prefix))
        })
        .cloned()
        .collect()
}

/// Payload-kind classifier used by `read_payload_units` and friends.
/// `quadlet/` accepts Quadlet-generator inputs; `systemd/` accepts native
/// systemd unit files. The split is structural — see research.md D6.
#[derive(Clone, Copy, Debug)]
enum PayloadKind {
    Quadlet,
    Systemd,
}

impl PayloadKind {
    fn name(self) -> &'static str {
        match self {
            PayloadKind::Quadlet => "quadlet",
            PayloadKind::Systemd => "systemd",
        }
    }

    fn accepts(self, qt: &QuadletType) -> bool {
        matches!(
            (self, qt),
            (PayloadKind::Quadlet, QuadletType::Container)
                | (PayloadKind::Quadlet, QuadletType::Volume)
                | (PayloadKind::Quadlet, QuadletType::Network)
                | (PayloadKind::Quadlet, QuadletType::Pod)
                | (PayloadKind::Systemd, QuadletType::Socket)
                | (PayloadKind::Systemd, QuadletType::Mount)
                | (PayloadKind::Systemd, QuadletType::Automount)
                | (PayloadKind::Systemd, QuadletType::Timer)
                | (PayloadKind::Systemd, QuadletType::Target)
                | (PayloadKind::Systemd, QuadletType::Path)
        )
    }
}

fn read_payload_units(
    payload_dir: &Path,
    kind: PayloadKind,
) -> Result<Vec<ArtifactSource>, RepoError> {
    let mut artifacts = Vec::new();
    if !payload_dir.exists() {
        return Ok(artifacts);
    }
    for entry in fs::read_dir(payload_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            // Drop-in directories are handled by `read_payload_dropins`.
            continue;
        }
        let (_, quadlet_type) =
            parse_quadlet_name(&file_name).map_err(|_| RepoError::InvalidPayloadKindFile {
                path: path.clone(),
                kind: kind.name(),
            })?;
        if !kind.accepts(&quadlet_type) {
            return Err(RepoError::InvalidPayloadKindFile {
                path,
                kind: kind.name(),
            });
        }
        let contents =
            fs::read_to_string(&path).map_err(|err| RepoError::Io(err.to_string()))?;
        artifacts.push(ArtifactSource {
            name: file_name,
            quadlet_type,
            contents,
            source_path: path.display().to_string(),
        });
    }
    Ok(artifacts)
}

fn read_payload_dropins(
    payload_dir: &Path,
    kind: PayloadKind,
) -> Result<Vec<DropInSource>, RepoError> {
    let mut dropins = Vec::new();
    if !payload_dir.exists() {
        return Ok(dropins);
    }
    for entry in fs::read_dir(payload_dir).map_err(|err| RepoError::Io(err.to_string()))? {
        let entry = entry.map_err(|err| RepoError::Io(err.to_string()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if let Some(target) = dropin_target_from_dir(&file_name) {
            // Validate that the target unit's extension matches THIS
            // payload kind. Without this check, a typo like
            // `services/<svc>/systemd/api.container.d/` would silently
            // attach to a `quadlet/api.container` base unit (validation
            // is by unit name, not subtree). Codex P2 on PR #28.
            let (_, target_quadlet_type) = parse_quadlet_name(&target).map_err(|_| {
                RepoError::InvalidPayloadKindFile {
                    path: path.clone(),
                    kind: kind.name(),
                }
            })?;
            if !kind.accepts(&target_quadlet_type) {
                return Err(RepoError::InvalidPayloadKindFile {
                    path,
                    kind: kind.name(),
                });
            }
            dropins.extend(read_dropins(&path, &target)?);
        } else {
            // A non-`.d` directory in a payload-kind subtree is rejected
            // rather than silently ignored: a typo like
            // `quadlet/foo.container.dropin/` would otherwise drop the
            // operator's drop-ins on the floor. Strict-layout contract
            // demands fail-fast (Codex P2 on PR #28).
            return Err(RepoError::LegacyArtifact(path));
        }
    }
    Ok(dropins)
}

/// Validates that an identifier (service id, host id, or
/// `config-root`) matches the documented pattern
/// `[A-Za-z0-9][A-Za-z0-9._-]*` AND does not begin with `_` or `.`
/// (FR-009 reserves those prefixes for future metadata).
///
/// The full-pattern check matters because `config-root` flows
/// directly into target paths (`/etc/<config-root>/...`). Without it,
/// a value like `foo/bar` would create destinations under `/etc/foo`
/// while observed-state scans collapsed to the first path segment,
/// causing unrelated `/etc/foo` files to be flagged for removal.
fn validate_id(name: &str) -> Result<(), RepoError> {
    if name.starts_with('_') || name.starts_with('.') {
        return Err(RepoError::ReservedName(name.to_string()));
    }
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return Err(RepoError::InvalidIdentifier(name.to_string())),
    };
    if !first.is_ascii_alphanumeric() {
        return Err(RepoError::InvalidIdentifier(name.to_string()));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
            return Err(RepoError::InvalidIdentifier(name.to_string()));
        }
    }
    Ok(())
}

/// Pre-walk check rejecting top-level legacy artifacts (FR-012). Called
/// at the entry point of every layered loader so the operator gets a
/// clear pointer at `scripts/migrate-legacy-source-repo.sh` before any
/// other validation produces noise.
fn validate_no_legacy_root_artifacts(repo_path: &Path) -> Result<(), RepoError> {
    let quadlets = repo_path.join("quadlets");
    if quadlets.exists() {
        return Err(RepoError::LegacyArtifact(quadlets));
    }
    Ok(())
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
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct HostYaml {
    host: String,
    services: Vec<String>,
}

fn git_fetch_revision(repo_path: &Path, revision: &str) -> Result<(), RepoError> {
    let parsed = parse_revision_expression(revision);
    if looks_like_commit_sha(parsed.fetch_ref) {
        // Commit SHAs (short or full) are not fetchable refspecs. The objects are
        // already present from the clone; skip the fetch and let checkout resolve them.
        return Ok(());
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("fetch")
        .arg("--depth")
        .arg(parsed.fetch_depth.to_string())
        .arg("origin")
        .arg(parsed.fetch_ref)
        .output()
        .map_err(|err| RepoError::GitFetchFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitFetchFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

fn git_checkout_revision(repo_path: &Path, revision: &str) -> Result<(), RepoError> {
    let parsed = parse_revision_expression(revision);
    let checkout_target = if looks_like_commit_sha(parsed.fetch_ref) {
        // Use the original revision expression directly so Git can resolve the
        // commit SHA (and any suffix like ~2) against the locally available objects.
        revision.to_string()
    } else {
        parsed.checkout_target
    };
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("checkout")
        .arg("--detach")
        .arg(checkout_target)
        .output()
        .map_err(|err| RepoError::GitCheckoutFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitCheckoutFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(())
}

fn looks_like_commit_sha(s: &str) -> bool {
    let len = s.len();
    (4..=40).contains(&len) && s.chars().all(|c| c.is_ascii_hexdigit())
}

struct ParsedRevisionExpression<'a> {
    fetch_ref: &'a str,
    checkout_target: String,
    fetch_depth: usize,
}

fn parse_revision_expression(revision: &str) -> ParsedRevisionExpression<'_> {
    let split_at = revision
        .find(['~', '^', ':', '@'])
        .unwrap_or(revision.len());
    let (fetch_ref, suffix) = revision.split_at(split_at);
    let fetch_ref = if fetch_ref.is_empty() {
        revision
    } else {
        fetch_ref
    };
    let checkout_target = if suffix.is_empty() {
        "FETCH_HEAD".to_string()
    } else {
        format!("FETCH_HEAD{suffix}")
    };
    ParsedRevisionExpression {
        fetch_ref,
        checkout_target,
        fetch_depth: required_fetch_depth(suffix),
    }
}

fn required_fetch_depth(suffix: &str) -> usize {
    let mut depth = 1usize;
    let mut chars = suffix.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '~' => {
                let mut digits = String::new();
                while let Some(next) = chars.peek() {
                    if next.is_ascii_digit() {
                        digits.push(*next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let steps = digits.parse::<usize>().unwrap_or(1);
                depth = depth.saturating_add(steps);
            }
            '^' => {
                while let Some(next) = chars.peek() {
                    if next.is_ascii_digit() {
                        chars.next();
                    } else {
                        break;
                    }
                }
                depth = depth.saturating_add(1);
            }
            _ => {}
        }
    }
    depth.max(1)
}

fn resolved_head_revision(repo_path: &Path) -> Result<String, RepoError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|err| RepoError::GitCheckoutFailed(err.to_string()))?;

    if !output.status.success() {
        return Err(RepoError::GitCheckoutFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        looks_like_commit_sha, parse_revision_expression, render_generated_automount_unit,
        render_generated_mount_unit, required_fetch_depth, NATIVE_UNIT_MANAGED_MARKER,
    };
    use crate::core::types::{MountDeclaration, MountVerificationMode};

    #[test]
    fn revision_expression_uses_fetch_head_for_plain_refs() {
        let parsed = parse_revision_expression("master");
        assert_eq!(parsed.fetch_ref, "master");
        assert_eq!(parsed.checkout_target, "FETCH_HEAD");
        assert_eq!(parsed.fetch_depth, 1);
    }

    #[test]
    fn revision_expression_supports_first_parent_suffix() {
        let parsed = parse_revision_expression("master~3");
        assert_eq!(parsed.fetch_ref, "master");
        assert_eq!(parsed.checkout_target, "FETCH_HEAD~3");
        assert_eq!(parsed.fetch_depth, 4);
    }

    #[test]
    fn revision_expression_supports_parent_suffix() {
        let parsed = parse_revision_expression("release^");
        assert_eq!(parsed.fetch_ref, "release");
        assert_eq!(parsed.checkout_target, "FETCH_HEAD^");
        assert_eq!(parsed.fetch_depth, 2);
    }

    #[test]
    fn required_fetch_depth_sums_parent_steps() {
        assert_eq!(required_fetch_depth(""), 1);
        assert_eq!(required_fetch_depth("~1"), 2);
        assert_eq!(required_fetch_depth("~5"), 6);
        assert_eq!(required_fetch_depth("^"), 2);
        assert_eq!(required_fetch_depth("~2^"), 4);
    }

    #[test]
    fn short_sha_is_recognized_as_commit_sha() {
        assert!(looks_like_commit_sha("454ac5f1"));
        assert!(looks_like_commit_sha("abcd1234"));
        assert!(looks_like_commit_sha("a1b2c3d4e5f6a1b2"));
    }

    #[test]
    fn full_sha_is_recognized_as_commit_sha() {
        // exactly 40 hex chars (SHA-1)
        assert!(looks_like_commit_sha(
            "454ac5f1deadbeefcafe00001111222233334444"
        ));
    }

    #[test]
    fn branch_and_tag_names_are_not_commit_shas() {
        assert!(!looks_like_commit_sha("master"));
        assert!(!looks_like_commit_sha("main"));
        assert!(!looks_like_commit_sha("v1.0.0"));
        assert!(!looks_like_commit_sha("feature/foo"));
        // Too short to be a meaningful SHA
        assert!(!looks_like_commit_sha("abc"));
        // Non-hex characters
        assert!(!looks_like_commit_sha("deadgood"));
    }

    #[test]
    fn revision_expression_short_sha_uses_direct_checkout() {
        // fetch_ref for a short SHA is still parsed correctly
        let parsed = parse_revision_expression("454ac5f1");
        assert_eq!(parsed.fetch_ref, "454ac5f1");
        assert_eq!(parsed.checkout_target, "FETCH_HEAD");
        // looks_like_commit_sha(fetch_ref) is what triggers the direct-checkout path
        assert!(looks_like_commit_sha(parsed.fetch_ref));
    }

    #[test]
    fn revision_expression_short_sha_with_suffix_parsed_correctly() {
        let parsed = parse_revision_expression("454ac5f1~2");
        assert_eq!(parsed.fetch_ref, "454ac5f1");
        assert_eq!(parsed.checkout_target, "FETCH_HEAD~2");
        assert_eq!(parsed.fetch_depth, 3);
        assert!(looks_like_commit_sha(parsed.fetch_ref));
    }

    #[test]
    fn generated_native_mount_units_include_management_marker() {
        let declaration = MountDeclaration {
            id: "var-lib-immich-media".to_string(),
            target_path: "/var/lib/immich/media".to_string(),
            source: "/usr/share/zoneinfo".to_string(),
            fstype: "none".to_string(),
            mount_options: vec!["bind".to_string(), "ro".to_string()],
            network_backed: false,
            automount: true,
            verification_mode: MountVerificationMode::UnitAndPath,
            prepared_path: None,
        };

        let mount = render_generated_mount_unit(&declaration);
        let automount = render_generated_automount_unit(&declaration);

        assert!(mount.contains(NATIVE_UNIT_MANAGED_MARKER));
        assert!(mount.contains("Options=bind,ro"));
        assert!(automount.contains(NATIVE_UNIT_MANAGED_MARKER));
        assert!(automount.contains("[Automount]"));
    }
}
