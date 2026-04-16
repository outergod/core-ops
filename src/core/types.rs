use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredState {
    pub repository_ref: String,
    pub revision_id: String,
    pub requested_repository: Option<String>,
    pub requested_ref: Option<String>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRunMode {
    Local,
    Ci,
    Debug,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRunOutcome {
    Passed,
    AssertionFailure,
    InfrastructureFailure,
    Timeout,
    HarnessError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationAssertionStatus {
    Passed,
    Failed,
    TimedOut,
    NotEvaluated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStepStatus {
    Pending,
    Passed,
    Failed,
    TimedOut,
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationArtifactCollectionStatus {
    Complete,
    Partial,
    Failed,
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
    #[serde(default)]
    pub detached: bool,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedObjectKind {
    GeneratedUnit,
    QuadletResource,
    Mount,
    Automount,
    RenderedArtifact,
}

impl ManagedObjectKind {
    pub fn resource_type(&self) -> &'static str {
        match self {
            ManagedObjectKind::GeneratedUnit => "service",
            ManagedObjectKind::QuadletResource => "resource",
            ManagedObjectKind::Mount => "mount",
            ManagedObjectKind::Automount => "automount",
            ManagedObjectKind::RenderedArtifact => "config",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedManagedObject {
    pub object_id: String,
    pub object_kind: ManagedObjectKind,
    pub material_fields: BTreeMap<String, String>,
    pub dependency_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedSnapshot {
    pub revision_id: Option<String>,
    pub scope_id: String,
    pub objects: Vec<NormalizedManagedObject>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeVerificationSignal {
    pub object_id: String,
    pub unit_name: Option<String>,
    pub active_state: Option<String>,
    pub details: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDependencyNode {
    pub object_id: String,
    pub object_kind: ManagedObjectKind,
    pub ordering_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyEdgeKind {
    Explicit,
    Implicit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDependencyEdge {
    pub from_object_id: String,
    pub to_object_id: String,
    pub edge_kind: DependencyEdgeKind,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDependencyGraph {
    pub nodes: Vec<SemanticDependencyNode>,
    pub edges: Vec<SemanticDependencyEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeterministicActionClass {
    Create,
    Update,
    Delete,
    Replace,
    Recover,
    Restart,
    NoOp,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftCategory {
    ExpectedChange,
    ExternalDrift,
    StaleResidue,
    RuntimeVariance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredDriftRecord {
    pub object_id: String,
    pub category: DriftCategory,
    pub comparison_basis: String,
    pub auto_action: bool,
    pub attention_required: bool,
    pub details: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicPlannedAction {
    pub object_id: String,
    pub classification: DeterministicActionClass,
    pub reason: String,
    pub dependency_context: Vec<String>,
    pub semantic_diff: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicReconciliationPlan {
    pub desired_revision_id: Option<String>,
    pub baseline_revision_id: Option<String>,
    pub requested_repository: Option<String>,
    pub requested_ref: Option<String>,
    pub last_applied_requested_repository: Option<String>,
    pub last_applied_requested_ref: Option<String>,
    pub scope_id: String,
    pub actions: Vec<DeterministicPlannedAction>,
    pub drift_records: Vec<StructuredDriftRecord>,
    pub graph: SemanticDependencyGraph,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackEligibility {
    Eligible,
    MissingSnapshot,
    IncompatibleScope,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedAppliedSnapshot {
    pub revision_id: String,
    pub scope_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_ref: Option<String>,
    pub snapshot: NormalizedSnapshot,
    pub retained: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackTargetCandidate {
    pub target_revision_id: String,
    pub scope_id: String,
    pub eligibility: RollbackEligibility,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceStatus {
    Success,
    Partial,
    Blocked,
    RepeatedFailure,
    Oscillation,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicConvergenceRecord {
    pub desired_revision_id: String,
    pub scope_id: String,
    pub status: ConvergenceStatus,
    pub attempt_count: u32,
    pub affected_objects: Vec<String>,
    pub completed_actions: Vec<String>,
    pub failed_actions: Vec<String>,
    pub can_continue: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicPersistedState {
    pub schema_version: u32,
    pub current_scope: String,
    pub retained_snapshots: Vec<RetainedAppliedSnapshot>,
    pub latest_convergence: Option<DeterministicConvergenceRecord>,
    pub latest_rollback_target: Option<RollbackTargetCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedObjectRef {
    pub resource_type: String,
    pub name: String,
    pub display_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionContext {
    pub target_revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_applied_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied_requested_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_applied_requested_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_revision: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseKind {
    DesiredChange,
    Drift,
    DependencyChange,
    DependencyFailure,
    BlockedPrerequisite,
    RuntimeVariance,
    RecoveryRequired,
    ReplacementRequired,
    RestartRequired,
    NoChange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cause {
    pub kind: CauseKind,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_object: Option<ManagedObjectRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyRelation {
    Prerequisite,
    Dependent,
    Blocker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyEdgeView {
    pub relation: DependencyRelation,
    pub object: ManagedObjectRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticDiffKind {
    LineBased,
    SemanticOnly,
    Replacement,
    Deletion,
    Creation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDiffView {
    pub kind: SemanticDiffKind,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unified_diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanEntryAction {
    Create,
    Update,
    Replace,
    Delete,
    Recover,
    Restart,
    NoOp,
    Blocked,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSummaryView {
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub skipped_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanEntry {
    pub object: ManagedObjectRef,
    pub action: PlanEntryAction,
    pub causes: Vec<Cause>,
    pub dependencies: Vec<DependencyEdgeView>,
    pub order_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<SemanticDiffView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOutputView {
    pub view_kind: String,
    pub revision_context: RevisionContext,
    pub summary: PlanSummaryView,
    pub entries: Vec<PlanEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPhaseKind {
    Resolution,
    GraphConstruction,
    Planning,
    Execution,
    ConvergenceCheck,
    FinalSummary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    Started,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPhaseEvent {
    pub phase: ApplyPhaseKind,
    pub state: PhaseState,
    pub sequence: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventKind {
    ObjectProgress,
    ObjectTerminal,
    ObjectBlocked,
    ObjectSkipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    Pending,
    Running,
    Created,
    Updated,
    Deleted,
    Recovered,
    Restarted,
    Unchanged,
    Failed,
    Blocked,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub object: ManagedObjectRef,
    pub event_kind: ExecutionEventKind,
    pub state: ExecutionState,
    pub sequence: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<PlanEntryAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<Cause>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<ApplyPhaseKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impacted_objects: Option<Vec<ManagedObjectRef>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyOutputView {
    pub view_kind: String,
    pub revision_context: RevisionContext,
    pub phases: Vec<ApplyPhaseEvent>,
    pub events: Vec<ExecutionEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<PlanSummaryView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultOutcome {
    Converged,
    ConvergedWithToleratedVariance,
    PartiallyApplied,
    Failed,
    NonConverging,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultSummaryView {
    pub changed_count: usize,
    pub failed_count: usize,
    pub blocked_count: usize,
    pub skipped_count: usize,
    pub unchanged_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultFinalState {
    Succeeded,
    Failed,
    Blocked,
    Skipped,
    NoOp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultEntry {
    pub object: ManagedObjectRef,
    pub final_state: ResultFinalState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<PlanEntryAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub causes: Option<Vec<Cause>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyEdgeView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<SemanticDiffView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultOutputView {
    pub view_kind: String,
    pub revision_context: RevisionContext,
    pub outcome: ResultOutcome,
    pub summary: ResultSummaryView,
    pub entries: Vec<ResultEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainOutputView {
    pub view_kind: String,
    pub revision_context: RevisionContext,
    pub object: ManagedObjectRef,
    pub action_or_outcome: String,
    pub causes: Vec<Cause>,
    pub dependencies: Vec<DependencyEdgeView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_context: Option<Vec<ExplainDependencyView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<SemanticDiffView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_coreops: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainDependencyView {
    pub relation: DependencyRelation,
    pub object: ManagedObjectRef,
    pub state: String,
    pub reason: String,
}
