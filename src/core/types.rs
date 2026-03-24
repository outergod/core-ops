use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredState {
    pub repository_ref: String,
    pub revision_id: String,
    pub workloads: Vec<Workload>,
    pub mount_declarations: Vec<MountDeclaration>,
    pub mount_dependencies: Vec<MountDependency>,
    pub managed_config_paths: Vec<String>,
    pub managed_config_roots: Vec<String>,
    pub invariants: Vec<Invariant>,
    pub boundaries: Boundaries,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceCatalog {
    pub services: BTreeMap<String, ServiceDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceDefinition {
    pub name: String,
    pub artifacts: Vec<ArtifactSource>,
    pub base_dropins: Vec<DropInSource>,
    pub config_files: Vec<ConfigFileSource>,
    pub mount_declarations: Vec<MountDeclaration>,
    pub service_mounts: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDeclaration {
    pub host: String,
    pub services: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostOverlaySet {
    pub host: String,
    pub overrides: Vec<DropInSource>,
    pub config_overrides: Vec<ConfigFileSource>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSource {
    pub name: String,
    pub quadlet_type: QuadletType,
    pub contents: String,
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropInSource {
    pub target: String,
    pub contents: String,
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigFileSource {
    pub target_path: String,
    pub contents: String,
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationInput {
    pub host: HostDeclaration,
    pub catalog: ServiceCatalog,
    pub overlays: HostOverlaySet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedArtifact {
    pub name: String,
    pub quadlet_type: QuadletType,
    pub contents: String,
    pub source_layers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedDropIn {
    pub target: String,
    pub file_name: String,
    pub contents: String,
    pub source_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedConfigFile {
    pub target_path: String,
    pub contents: String,
    pub source_layers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountDeclaration {
    pub id: String,
    pub target_path: String,
    pub source: String,
    pub fstype: String,
    pub mount_options: Vec<String>,
    pub network_backed: bool,
    pub automount: bool,
    pub verification_mode: MountVerificationMode,
    pub ownership_scope: Vec<String>,
    pub prepared_path: Option<PreparedTargetPath>,
}

impl MountDeclaration {
    pub fn mount_unit_name(&self) -> String {
        mount_unit_name_for_path(&self.target_path)
    }

    pub fn automount_unit_name(&self) -> Option<String> {
        self.automount
            .then(|| automount_unit_name_for_path(&self.target_path))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountDependency {
    pub service_name: String,
    pub mount_ids: Vec<String>,
    pub consumed_paths: Vec<String>,
    pub path_dependency_mode: PathDependencyMode,
    pub unit_dependency_mode: UnitDependencyMode,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedTargetPath {
    pub path: String,
    pub create_if_missing: bool,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub mode: Option<String>,
    pub service_consumed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedUnitSet {
    pub declaration_id: String,
    pub mount_unit_name: String,
    pub automount_unit_name: Option<String>,
    pub service_dependency_edits: Vec<ServiceDependencyEdit>,
    pub removal_candidates: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceDependencyEdit {
    pub service_name: String,
    pub requires_mounts_for: Vec<String>,
    pub after_units: Vec<String>,
    pub requires_units: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MountReconciliationResult {
    pub mount_id: String,
    pub validation_status: MountValidationStatus,
    pub activation_status: MountActivationStatus,
    pub verification_status: MountVerificationStatus,
    pub dependent_service_effect: DependentServiceEffect,
    pub failure_reason: Option<String>,
    pub removal_result: MountRemovalResult,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountVerificationMode {
    UnitAndPath,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathDependencyMode {
    RequiresMountsFor,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitDependencyMode {
    AfterAndRequires,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountValidationStatus {
    Valid,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountActivationStatus {
    NotApplied,
    Active,
    Degraded,
    Failed,
    Removing,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountVerificationStatus {
    Verified,
    Unverified,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DependentServiceEffect {
    None,
    Blocked,
    Degraded,
    StoppedForRemoval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountRemovalResult {
    NotRequested,
    Removed,
    Busy,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workload {
    pub name: String,
    pub quadlet_type: QuadletType,
    pub quadlet_contents: String,
    pub systemd_unit_name: String,
    pub enabled_state: EnabledState,
    pub restart_policy: RestartPolicy,
}

impl Workload {
    pub fn key(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedState {
    pub observed_revision_id: Option<String>,
    pub units: Vec<ObservedUnit>,
    pub workloads: Vec<Workload>,
    pub last_reconcile_id: Option<String>,
    pub host_info: Option<HostInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedUnit {
    pub unit_name: String,
    pub active_state: UnitActiveState,
    pub enabled_state: EnabledState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInfo {
    pub hostname: String,
    pub os_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationPlan {
    pub plan_id: String,
    pub desired_revision_id: String,
    pub observed_revision_id: Option<String>,
    pub actions: Vec<PlanAction>,
    pub safety_checks: Vec<SafetyCheck>,
    pub expected_outcomes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanAction {
    pub action_type: PlanActionType,
    pub target: String,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcileRun {
    pub run_id: String,
    pub mode: ReconcileMode,
    pub status: RunStatus,
    pub failure_class: Option<FailureClass>,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRecord {
    pub record_id: String,
    pub run_id: String,
    pub diffs: Vec<DiffItem>,
    pub plan_summary: String,
    pub actions_applied: Vec<PlanAction>,
    pub verification_results: Vec<VerificationResult>,
    pub operator_messages: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffItem {
    pub name: String,
    pub kind: DiffKind,
    pub desired: Option<Workload>,
    pub observed: Option<Workload>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Add,
    Remove,
    Change,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuadletType {
    Container,
    Socket,
    SocketDropIn,
    ConfigFile,
    Mount,
    Automount,
    Pod,
    Volume,
    Network,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnabledState {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestartPolicy {
    Always,
    OnFailure,
    Never,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitActiveState {
    Active,
    Inactive,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanActionType {
    PreparePath,
    WriteQuadlet,
    RemoveQuadlet,
    EnableUnit,
    DisableUnit,
    ReloadSystemd,
    StartUnit,
    RestartUnit,
    StopUnit,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SafetyCheck {
    BoundariesDeclared,
    SupportedQuadletTypes,
    DeterministicPlan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invariant {
    BoundariesDeclared,
    DeterministicPlan,
    IdempotentApply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boundaries {
    pub scopes: Vec<BoundaryScope>,
}

impl Boundaries {
    pub fn has_scope(&self, scope: BoundaryScope) -> bool {
        self.scopes.contains(&scope)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundaryScope {
    QuadletSystemd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FailureClass {
    Validation,
    Plan,
    Apply,
    Verify,
    Transient,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconcileMode {
    Plan,
    Apply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunStatus {
    Success,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationResult {
    pub target: String,
    pub status: VerificationStatus,
    pub details: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Success,
    Failure,
}

pub struct RunLockGuard {
    pub lock_id: String,
}

pub trait RunLock {
    fn acquire(&self) -> Result<RunLockGuard, crate::core::errors::RunLockError>;
    fn release(&self, guard: RunLockGuard) -> Result<(), crate::core::errors::RunLockError>;
}

pub fn index_workloads(workloads: &[Workload]) -> BTreeMap<String, Workload> {
    let mut map = BTreeMap::new();
    for workload in workloads {
        map.insert(workload.name.clone(), workload.clone());
    }
    map
}

pub const PERSISTED_PROVENANCE_SCHEMA_VERSION: u32 = 1;

fn escape_path_for_unit(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "-".to_string();
    }

    let mut escaped = String::new();
    for ch in trimmed.chars() {
        match ch {
            '/' => escaped.push('-'),
            c if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':') => escaped.push(c),
            c => escaped.push_str(&format!("\\x{:02x}", c as u32)),
        }
    }
    escaped
}

pub fn mount_unit_name_for_path(path: &str) -> String {
    format!("{}.mount", escape_path_for_unit(path))
}

pub fn automount_unit_name_for_path(path: &str) -> String {
    format!("{}.automount", escape_path_for_unit(path))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProvenanceState {
    pub schema_version: u32,
    pub controller: ControllerProvenance,
    pub desired_state: DesiredStateProvenance,
    pub reconciliation: ReconciliationProvenance,
}

impl PersistedProvenanceState {
    pub fn is_supported_schema(&self) -> bool {
        self.schema_version == PERSISTED_PROVENANCE_SCHEMA_VERSION
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerProvenance {
    pub version: Option<String>,
    pub revision: Option<String>,
    pub build_time: Option<String>,
    pub tree_state: TreeState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredStateProvenance {
    pub repository: String,
    pub requested_ref: String,
    pub last_observed_revision: Option<String>,
    pub last_observed_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationProvenance {
    pub generation: u64,
    pub status: ReconciliationStatus,
    pub running: bool,
    pub last_attempted_revision: Option<String>,
    pub last_applied_revision: Option<String>,
    pub last_started_at: Option<String>,
    pub last_finished_at: Option<String>,
    pub attempted_observed_divergence: Option<RevisionDivergence>,
}

impl ReconciliationProvenance {
    pub fn is_valid(&self) -> bool {
        match self.status {
            ReconciliationStatus::NeverRun => {
                if self.running
                    || self.generation != 0
                    || self.last_attempted_revision.is_some()
                    || self.last_applied_revision.is_some()
                    || self.last_started_at.is_some()
                    || self.last_finished_at.is_some()
                    || self.attempted_observed_divergence.is_some()
                {
                    return false;
                }
            }
            ReconciliationStatus::InProgress => {
                if !self.running
                    || self.last_started_at.is_none()
                    || self.last_finished_at.is_some()
                {
                    return false;
                }
            }
            ReconciliationStatus::Success => {
                if self.running
                    || self.last_started_at.is_none()
                    || self.last_finished_at.is_none()
                    || self.last_attempted_revision.is_none()
                    || self.last_applied_revision != self.last_attempted_revision
                {
                    return false;
                }
            }
            ReconciliationStatus::Failed => {
                if self.running
                    || self.last_started_at.is_none()
                    || self.last_finished_at.is_none()
                    || self.last_attempted_revision.is_none()
                {
                    return false;
                }
            }
        }
        if let Some(divergence) = &self.attempted_observed_divergence {
            if divergence.observed_revision == divergence.attempted_revision {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDivergence {
    pub observed_revision: String,
    pub attempted_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationStatus {
    NeverRun,
    InProgress,
    Success,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeState {
    Clean,
    Dirty,
    Unknown,
}
