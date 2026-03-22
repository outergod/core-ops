use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredState {
    pub repository_ref: String,
    pub revision_id: String,
    pub workloads: Vec<Workload>,
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
