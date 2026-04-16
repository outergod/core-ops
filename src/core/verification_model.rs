use crate::core::errors::CoreError;
use crate::core::types::{
    FailureClass, VerificationArtifactCollectionStatus, VerificationAssertionStatus,
    VerificationRunMode, VerificationRunOutcome, VerificationStepStatus,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const SUPPORTED_ENVIRONMENT_PROFILE: &str = "single-blessed-vm";
const SUPPORTED_TIMEOUT_PROFILE: &str = "standard";
const SUPPORTED_ARTIFACT_PROFILE: &str = "standard";
const SUPPORTED_BACKEND: &str = "approved-libvirt";
const SUPPORTED_GUEST_IMAGE: &str = "blessed-coreops-guest";
const SUPPORTED_NETWORK_POLICY: &str = "isolated";
pub const VERIFICATION_READINESS_MARKER: &str = "CORE_OPS_VERIFY_READY";
pub const VERIFICATION_READINESS_SERVICE_NAME: &str = "core-ops-verify-ready.service";
pub const VERIFICATION_READINESS_SCRIPT_PATH: &str = "/usr/local/bin/core-ops-verify-ready";
const REQUIRED_ALWAYS_COLLECTED: [&str; 5] = [
    "scenario-definition",
    "harness-log",
    "console-log",
    "coreops-output",
    "assertion-results",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationScenarioSource {
    Accepted,
    Candidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationScenarioClass {
    Convergence,
    DriftCorrection,
    Idempotency,
    UpgradeTransition,
    RebootResilience,
    ExplainApplyConsistency,
    RegressionDetection,
    ReleaseGateSuccess,
    ReleaseGateFailure,
    VerificationEnvironmentIdentity,
    VersionIdentityVisibility,
    InstallationPathValidation,
    OperatorVerificationFlow,
    OperatorVerificationReproducibility,
    ColdStartDistributionValidation,
    DistributionArtifactValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStepType {
    Boot,
    WaitReady,
    #[serde(alias = "run_coreops")]
    CoreopsAction,
    #[serde(alias = "run_guest_command")]
    GuestCommand,
    MutateState,
    Reboot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStepTarget {
    Guest,
    Harness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCandidateReviewStatus {
    Generated,
    NeedsReview,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCoreOpsActionKind {
    Apply,
    Explain,
    Init,
    Plan,
    Status,
    Agent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEnvironmentSelection {
    pub profile: String,
    #[serde(default)]
    pub overrides: Option<VerificationEnvironmentOverride>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerificationEnvironmentOverride {
    #[serde(default)]
    pub image_version: Option<String>,
    #[serde(default)]
    pub readiness_checks: Option<Vec<String>>,
    #[serde(default)]
    pub connection_profile: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationEnvironmentSpec {
    pub backend_family: String,
    pub guest_image_family: String,
    pub image_version: String,
    pub network_policy: String,
    pub bootstrap_policy: String,
    pub guest: VerificationGuestSpec,
    #[serde(default)]
    pub default_policy: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationGuestSpec {
    pub guest_name: String,
    pub cpu_profile: String,
    pub memory_profile: String,
    pub disk_overlay_policy: String,
    pub readiness_checks: Vec<String>,
    pub connection_profile: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationHarnessPolicyOverride {
    #[serde(default)]
    pub timeout_profile: Option<String>,
    #[serde(default)]
    pub timeouts: Option<VerificationTimeoutPolicy>,
    #[serde(default)]
    pub artifact_profile: Option<String>,
    #[serde(default)]
    pub artifact_policy: Option<VerificationArtifactPolicy>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationFixtureSet {
    pub repo_fixture: String,
    #[serde(default)]
    pub repository_evolution: Option<VerificationRepositoryEvolution>,
    #[serde(default)]
    pub config_inputs: Vec<String>,
    #[serde(default)]
    pub test_data: Vec<String>,
    pub revision_under_test: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerificationRepositoryEvolution {
    #[serde(default)]
    pub history_fixture: Option<String>,
    #[serde(default)]
    pub revisions: Vec<String>,
    #[serde(default)]
    pub states: Vec<String>,
    #[serde(default)]
    pub transition_expectations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCoreOpsAction {
    pub action: VerificationCoreOpsActionKind,
    #[serde(default)]
    pub repository_source: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub output_contract: Option<String>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationScenarioStep {
    pub step_id: String,
    pub step_type: VerificationStepType,
    pub target: VerificationStepTarget,
    #[serde(default)]
    pub action: Option<VerificationCoreOpsAction>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default, alias = "command_or_action")]
    pub legacy_command_or_action: Option<String>,
    #[serde(default)]
    pub expected_exit_behavior: Option<String>,
    #[serde(default)]
    pub timeout_override: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationAssertionSpec {
    pub assertion_id: String,
    pub assertion_type: String,
    pub target: String,
    pub expected_state: String,
    pub failure_message: String,
    #[serde(default)]
    pub artifact_hints: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationTimeoutPolicy {
    #[serde(default)]
    pub per_step_defaults: BTreeMap<String, String>,
    pub scenario_timeout: String,
    pub readiness_timeout: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationArtifactPolicy {
    #[serde(default)]
    pub always_collect: Vec<String>,
    #[serde(default)]
    pub collect_on_failure: Vec<String>,
    pub retain_environment_in_debug: bool,
    pub export_format: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationScenarioDefinition {
    pub scenario_id: String,
    pub title: String,
    pub description: String,
    pub scenario_classes: Vec<VerificationScenarioClass>,
    pub source: VerificationScenarioSource,
    pub behavioral_claim: String,
    pub rationale: String,
    pub environment: VerificationEnvironmentSelection,
    pub fixtures: VerificationFixtureSet,
    pub steps: Vec<VerificationScenarioStep>,
    pub assertions: Vec<VerificationAssertionSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_outcome: Option<VerificationRunOutcome>,
    #[serde(default)]
    pub policy_overrides: Option<VerificationHarnessPolicyOverride>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCandidateScenario {
    pub candidate_id: String,
    pub proposed_definition: VerificationScenarioDefinition,
    #[serde(default)]
    pub generation_inputs: Vec<String>,
    #[serde(default)]
    pub validation_findings: Vec<String>,
    pub review_status: VerificationCandidateReviewStatus,
}

impl VerificationCandidateScenario {
    pub fn mark_rejected(&mut self, finding: impl Into<String>) {
        self.review_status = VerificationCandidateReviewStatus::Rejected;
        self.validation_findings.push(finding.into());
    }

    pub fn mark_needs_review(&mut self) {
        if self.review_status == VerificationCandidateReviewStatus::Generated {
            self.review_status = VerificationCandidateReviewStatus::NeedsReview;
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationAssertionResult {
    pub assertion_id: String,
    pub status: VerificationAssertionStatus,
    #[serde(default)]
    pub observed_value: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReadinessRecord {
    pub run_id: String,
    pub token: String,
    pub ip: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReadinessExpectation {
    pub run_id: String,
    pub token: String,
    pub marker: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReadinessRejectionKind {
    Stale,
    Malformed,
    DuplicateCurrentRun,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReadinessRejection {
    pub kind: VerificationReadinessRejectionKind,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_line: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReadinessEvidence {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_record: Option<VerificationReadinessRecord>,
    #[serde(default)]
    pub rejected_records: Vec<VerificationReadinessRejection>,
    pub final_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_summary: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationGuestReadinessPayload {
    pub run_id: String,
    pub token: String,
    pub console_marker: String,
    pub service_name: String,
    pub script_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReadinessAcquisition {
    pub guest: LibvirtGuestHandle,
    pub evidence: VerificationReadinessEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationStepResult {
    pub step_id: String,
    pub step_type: VerificationStepType,
    pub status: VerificationStepStatus,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub stderr: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationScenarioOutcome {
    pub scenario_id: String,
    pub revision_under_test: String,
    pub outcome: VerificationRunOutcome,
    #[serde(default)]
    pub step_results: Vec<VerificationStepResult>,
    #[serde(default)]
    pub assertion_results: Vec<VerificationAssertionResult>,
    #[serde(default)]
    pub failure_summary: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationRevisionSelectionBasis {
    SingleScenario,
    AcceptedCorpus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationArtifactManifestEntry {
    pub logical_name: String,
    pub relative_path: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationArtifactBundle {
    pub bundle_path: String,
    #[serde(default)]
    pub manifest_entries: Vec<VerificationArtifactManifestEntry>,
    #[serde(default)]
    pub always_collected_entries: Vec<String>,
    #[serde(default)]
    pub failure_specific_entries: Vec<String>,
    pub environment_retained: bool,
    #[serde(default = "default_collection_status")]
    pub collection_status: VerificationArtifactCollectionStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRun {
    pub run_id: String,
    pub mode: VerificationRunMode,
    pub revision_selection_basis: VerificationRevisionSelectionBasis,
    pub revision_under_test: String,
    pub controller_version: String,
    #[serde(default)]
    pub scenario_refs: Vec<String>,
    pub workspace_path: String,
    pub started_at: String,
    pub completed_at: String,
    pub overall_outcome: VerificationRunOutcome,
    pub artifact_bundle: VerificationArtifactBundle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationExecutionPlan {
    pub scenario_id: String,
    pub run_id: String,
    pub mode: VerificationRunMode,
    pub step_sequence: Vec<VerificationPlannedStep>,
    pub retain_environment: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationPlannedStep {
    pub step_id: String,
    pub step_type: VerificationStepType,
    pub target: VerificationStepTarget,
    pub effective_timeout: String,
    #[serde(default)]
    pub command_or_action: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRuntimeBindings {
    pub repo_path: String,
    pub core_ops_binary: String,
    pub quadlet_dir: String,
    pub systemd_unit_dir: String,
    pub state_file: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRunView {
    pub view_kind: String,
    pub run_id: String,
    pub mode: VerificationRunMode,
    pub controller_version: String,
    pub revision_selection_basis: VerificationRevisionSelectionBasis,
    pub revision_under_test: String,
    pub started_at: String,
    pub completed_at: String,
    pub scenario_id: String,
    pub title: String,
    pub overall_outcome: VerificationRunOutcome,
    pub artifact_bundle: VerificationArtifactBundle,
    pub environment_retained: bool,
    #[serde(default)]
    pub step_results: Vec<VerificationStepResult>,
    #[serde(default)]
    pub assertion_results: Vec<VerificationAssertionResult>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub failure_summary: Option<String>,
    #[serde(default)]
    pub regression_summary: Option<String>,
    #[serde(default)]
    pub promotion_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_evidence: Option<VerificationReadinessEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationRunArtifacts {
    pub bundle: VerificationArtifactBundle,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibvirtGuestHandle {
    pub guest_name: String,
    pub domain_name: String,
    pub ssh_target: String,
    pub connection_uri: String,
    pub workspace_root: String,
    pub env_backed: bool,
    pub network_mode: Option<String>,
    pub vm_host: Option<String>,
    pub ssh_user: Option<String>,
    pub ignition_path: Option<String>,
    pub local_butane_path: Option<String>,
    pub local_ignition_path: Option<String>,
    pub volume_name: Option<String>,
    pub assigned_ip: Option<String>,
    pub lease_path: Option<String>,
    pub rendered_network_config: Option<String>,
    pub serial_log_path: Option<String>,
    pub qemu_launch_log_path: Option<String>,
    pub readiness_payload: Option<VerificationGuestReadinessPayload>,
    pub readiness_evidence: Option<VerificationReadinessEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuestCommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl VerificationScenarioDefinition {
    pub fn effective_environment(&self) -> Result<VerificationEnvironmentSpec, CoreError> {
        if self.environment.profile != SUPPORTED_ENVIRONMENT_PROFILE {
            return Err(CoreError::new(
                FailureClass::Validation,
                format!(
                    "environment.profile must resolve to the approved v1 profile `{SUPPORTED_ENVIRONMENT_PROFILE}`"
                ),
            ));
        }

        let mut profile = default_environment_profile();
        if let Some(overrides) = &self.environment.overrides {
            if let Some(image_version) = &overrides.image_version {
                profile.image_version = image_version.clone();
            }
            if let Some(readiness_checks) = &overrides.readiness_checks {
                profile.guest.readiness_checks = readiness_checks.clone();
            }
            if let Some(connection_profile) = &overrides.connection_profile {
                profile.guest.connection_profile = connection_profile.clone();
            }
        }
        Ok(profile)
    }

    pub fn effective_timeouts(&self) -> Result<VerificationTimeoutPolicy, CoreError> {
        let mut timeouts = default_timeout_policy();
        if let Some(overrides) = &self.policy_overrides {
            if let Some(timeout_profile) = &overrides.timeout_profile {
                if timeout_profile != SUPPORTED_TIMEOUT_PROFILE {
                    return Err(CoreError::new(
                        FailureClass::Validation,
                        format!(
                            "policy_overrides.timeout_profile must resolve to `{SUPPORTED_TIMEOUT_PROFILE}`"
                        ),
                    ));
                }
            }
            if let Some(policy) = &overrides.timeouts {
                merge_timeouts(&mut timeouts, policy);
            }
        }
        Ok(timeouts)
    }

    pub fn effective_artifact_policy(&self) -> Result<VerificationArtifactPolicy, CoreError> {
        let mut policy = default_artifact_policy();
        if let Some(overrides) = &self.policy_overrides {
            if let Some(artifact_profile) = &overrides.artifact_profile {
                if artifact_profile != SUPPORTED_ARTIFACT_PROFILE {
                    return Err(CoreError::new(
                        FailureClass::Validation,
                        format!(
                            "policy_overrides.artifact_profile must resolve to `{SUPPORTED_ARTIFACT_PROFILE}`"
                        ),
                    ));
                }
            }
            if let Some(artifact_policy) = &overrides.artifact_policy {
                merge_artifact_policy(&mut policy, artifact_policy);
            }
        }
        Ok(policy)
    }

    pub fn render_step_command(
        &self,
        step: &VerificationScenarioStep,
        bindings: Option<&VerificationRuntimeBindings>,
    ) -> Result<Option<String>, CoreError> {
        let default_bindings = VerificationRuntimeBindings {
            repo_path: self.fixtures.repo_fixture.clone(),
            core_ops_binary: "core-ops".to_string(),
            quadlet_dir: "/etc/containers/systemd".to_string(),
            systemd_unit_dir: "/etc/systemd/system".to_string(),
            state_file: "/var/lib/core-ops/status.json".to_string(),
        };
        let bindings = bindings.unwrap_or(&default_bindings);
        let rendered = match step.step_type {
            VerificationStepType::Boot | VerificationStepType::WaitReady => None,
            VerificationStepType::CoreopsAction => {
                let action = step.action.as_ref().ok_or_else(|| {
                    CoreError::new(
                        FailureClass::Validation,
                        format!("step `{}` requires a structured action", step.step_id),
                    )
                })?;
                Some(render_coreops_action(action, bindings))
            }
            VerificationStepType::GuestCommand
            | VerificationStepType::MutateState
            | VerificationStepType::Reboot => step
                .command
                .clone()
                .or_else(|| step.legacy_command_or_action.clone())
                .or_else(|| {
                    if step.step_type == VerificationStepType::Reboot {
                        Some("sudo systemctl reboot".to_string())
                    } else {
                        None
                    }
                }),
        };
        Ok(rendered)
    }
}

pub fn parse_scenario_definition(input: &str) -> Result<VerificationScenarioDefinition, CoreError> {
    let scenario =
        serde_yaml::from_str::<VerificationScenarioDefinition>(input).map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid verification scenario yaml: {err}"),
            )
        })?;
    validate_scenario_definition(&scenario)?;
    Ok(scenario)
}

pub fn load_scenario_definition(path: &Path) -> Result<VerificationScenarioDefinition, CoreError> {
    let raw = fs::read_to_string(path).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read scenario {}: {err}", path.display()),
        )
    })?;
    parse_scenario_definition(&raw)
}

pub fn validate_scenario_definition(
    scenario: &VerificationScenarioDefinition,
) -> Result<(), CoreError> {
    if scenario.scenario_id.trim().is_empty() {
        return validation_error("scenario_id must be present");
    }
    if scenario.steps.is_empty() {
        return validation_error("scenario must contain at least one step");
    }
    if scenario.assertions.is_empty() {
        return validation_error("scenario must contain at least one assertion");
    }
    if scenario.scenario_classes.is_empty() {
        return validation_error("scenario must declare at least one scenario class");
    }
    if scenario.behavioral_claim.trim().is_empty() {
        return validation_error("behavioral_claim must be present");
    }
    if scenario.rationale.trim().is_empty() {
        return validation_error("rationale must be present");
    }

    let environment = scenario.effective_environment()?;
    if environment.backend_family != SUPPORTED_BACKEND {
        return validation_error("resolved environment backend_family must use the approved v1 backend");
    }
    if environment.guest_image_family != SUPPORTED_GUEST_IMAGE {
        return validation_error("resolved environment guest_image_family must use the approved v1 image family");
    }
    if environment.network_policy != SUPPORTED_NETWORK_POLICY {
        return validation_error("resolved environment network_policy must remain isolated in v1");
    }
    if environment.guest.readiness_checks.is_empty() {
        return validation_error("resolved environment guest must declare at least one readiness check");
    }

    if scenario.fixtures.revision_under_test.trim().is_empty() {
        return validation_error("fixtures.revision_under_test must be explicit");
    }
    if let Some(repository_evolution) = &scenario.fixtures.repository_evolution {
        if repository_evolution.revisions.is_empty() {
            return validation_error("fixtures.repository_evolution.revisions must not be empty when repository evolution is declared");
        }
    }

    let timeouts = scenario.effective_timeouts()?;
    if timeouts.scenario_timeout.trim().is_empty() || timeouts.readiness_timeout.trim().is_empty() {
        return validation_error("effective timeout policy must define scenario_timeout and readiness_timeout");
    }

    let artifact_policy = scenario.effective_artifact_policy()?;
    for required in REQUIRED_ALWAYS_COLLECTED {
        if !artifact_policy
            .always_collect
            .iter()
            .any(|entry| entry == required)
        {
            return validation_error(format!(
                "artifact policy missing required always_collect entry `{required}`"
            ));
        }
    }

    for step in &scenario.steps {
        match step.step_type {
            VerificationStepType::CoreopsAction => {
                if step.action.is_none() {
                    return validation_error(format!(
                        "step `{}` with step_type=coreops_action must define action",
                        step.step_id
                    ));
                }
            }
            VerificationStepType::GuestCommand | VerificationStepType::MutateState => {
                if step.command.is_none() && step.legacy_command_or_action.is_none() {
                    return validation_error(format!(
                        "step `{}` must define command",
                        step.step_id
                    ));
                }
            }
            VerificationStepType::Boot | VerificationStepType::WaitReady | VerificationStepType::Reboot => {}
        }
    }

    let step_ids = scenario
        .steps
        .iter()
        .map(|step| step.step_id.as_str())
        .collect::<Vec<_>>();
    for assertion in &scenario.assertions {
        if !is_supported_assertion_type(&assertion.assertion_type) {
            return validation_error(format!(
                "assertion `{}` uses unsupported assertion_type `{}`",
                assertion.assertion_id, assertion.assertion_type
            ));
        }
        if assertion_requires_step_target(&assertion.assertion_type)
            && !step_ids.iter().any(|step_id| step_id == &assertion.target.as_str())
        {
            return validation_error(format!(
                "assertion `{}` targets unknown step `{}`",
                assertion.assertion_id, assertion.target
            ));
        }
    }

    Ok(())
}

pub fn build_artifact_bundle(
    bundle_path: impl Into<String>,
    manifest_entries: Vec<VerificationArtifactManifestEntry>,
    environment_retained: bool,
    collection_status: VerificationArtifactCollectionStatus,
) -> VerificationArtifactBundle {
    let always_collected_entries = manifest_entries
        .iter()
        .filter(|entry| entry.required)
        .map(|entry| entry.logical_name.clone())
        .collect();

    VerificationArtifactBundle {
        bundle_path: bundle_path.into(),
        manifest_entries,
        always_collected_entries,
        failure_specific_entries: Vec::new(),
        environment_retained,
        collection_status,
    }
}

pub fn default_environment_profile() -> VerificationEnvironmentSpec {
    VerificationEnvironmentSpec {
        backend_family: SUPPORTED_BACKEND.to_string(),
        guest_image_family: SUPPORTED_GUEST_IMAGE.to_string(),
        image_version: "2026-04-01".to_string(),
        network_policy: SUPPORTED_NETWORK_POLICY.to_string(),
        bootstrap_policy: "approved-bootstrap".to_string(),
        guest: VerificationGuestSpec {
            guest_name: "primary".to_string(),
            cpu_profile: "small".to_string(),
            memory_profile: "small".to_string(),
            disk_overlay_policy: "disposable".to_string(),
            readiness_checks: vec!["ssh-ready".to_string()],
            connection_profile: "default-ssh".to_string(),
        },
        default_policy: Some(SUPPORTED_TIMEOUT_PROFILE.to_string()),
    }
}

pub fn default_timeout_policy() -> VerificationTimeoutPolicy {
    VerificationTimeoutPolicy {
        per_step_defaults: BTreeMap::from([
            ("boot".to_string(), "300s".to_string()),
            ("wait_ready".to_string(), "180s".to_string()),
            ("coreops_action".to_string(), "300s".to_string()),
            ("guest_command".to_string(), "180s".to_string()),
            ("mutate_state".to_string(), "180s".to_string()),
            ("reboot".to_string(), "300s".to_string()),
        ]),
        scenario_timeout: "1200s".to_string(),
        readiness_timeout: "180s".to_string(),
    }
}

pub fn default_artifact_policy() -> VerificationArtifactPolicy {
    VerificationArtifactPolicy {
        always_collect: REQUIRED_ALWAYS_COLLECTED
            .iter()
            .map(|entry| entry.to_string())
            .collect(),
        collect_on_failure: vec![
            "journal-excerpts".to_string(),
            "explain-output".to_string(),
            "relevant-files".to_string(),
        ],
        retain_environment_in_debug: true,
        export_format: "directory".to_string(),
    }
}

fn merge_timeouts(base: &mut VerificationTimeoutPolicy, overrides: &VerificationTimeoutPolicy) {
    if !overrides.scenario_timeout.trim().is_empty() {
        base.scenario_timeout = overrides.scenario_timeout.clone();
    }
    if !overrides.readiness_timeout.trim().is_empty() {
        base.readiness_timeout = overrides.readiness_timeout.clone();
    }
    for (key, value) in &overrides.per_step_defaults {
        base.per_step_defaults.insert(key.clone(), value.clone());
    }
}

fn merge_artifact_policy(
    base: &mut VerificationArtifactPolicy,
    overrides: &VerificationArtifactPolicy,
) {
    if !overrides.always_collect.is_empty() {
        base.always_collect = overrides.always_collect.clone();
    }
    if !overrides.collect_on_failure.is_empty() {
        base.collect_on_failure = overrides.collect_on_failure.clone();
    }
    base.retain_environment_in_debug = overrides.retain_environment_in_debug;
    if !overrides.export_format.trim().is_empty() {
        base.export_format = overrides.export_format.clone();
    }
}

fn render_coreops_action(
    action: &VerificationCoreOpsAction,
    bindings: &VerificationRuntimeBindings,
) -> String {
    let mut command = format!("sudo {}", bindings.core_ops_binary);
    command.push(' ');
    command.push_str(action_label(action.action));
    match action.action {
        VerificationCoreOpsActionKind::Init => {
            command.push(' ');
            command.push_str(repository_source_arg(
                &action.repository_source,
                &bindings.repo_path,
            ));
            command.push(' ');
            command.push_str(&action.revision);
            if action.force {
                command.push_str(" --force");
            }
        }
        VerificationCoreOpsActionKind::Apply | VerificationCoreOpsActionKind::Plan => {
            if let Some(host) = &action.host {
                command.push_str(" --host ");
                command.push_str(host);
            }
            command.push_str(" --quadlet-dir ");
            command.push_str(&bindings.quadlet_dir);
            command.push_str(" --systemd-unit-dir ");
            command.push_str(&bindings.systemd_unit_dir);
            append_interface_flags(&mut command, action, true, true);
        }
        VerificationCoreOpsActionKind::Explain => {
            if let Some(object) = &action.object {
                command.push(' ');
                command.push_str(object);
            }
            if let Some(host) = &action.host {
                command.push_str(" --host ");
                command.push_str(host);
            }
            command.push_str(" --quadlet-dir ");
            command.push_str(&bindings.quadlet_dir);
            command.push_str(" --systemd-unit-dir ");
            command.push_str(&bindings.systemd_unit_dir);
            append_interface_flags(&mut command, action, true, false);
        }
        VerificationCoreOpsActionKind::Status => {}
        VerificationCoreOpsActionKind::Agent => {
            if let Some(host) = &action.host {
                command.push_str(" --host ");
                command.push_str(host);
            }
            command.push_str(" --quadlet-dir ");
            command.push_str(&bindings.quadlet_dir);
            command.push_str(" --systemd-unit-dir ");
            command.push_str(&bindings.systemd_unit_dir);
        }
    }

    command
}

fn append_interface_flags(
    command: &mut String,
    action: &VerificationCoreOpsAction,
    supports_json: bool,
    supports_verbose: bool,
) {
    let mut json_requested = false;
    if let Some(mode) = &action.mode {
        match mode.as_str() {
            "json" if supports_json => {
                command.push_str(" --json");
                json_requested = true;
            }
            "verbose" if supports_verbose => command.push_str(" --verbose"),
            "humane" => {}
            _ => {}
        }
    }
    if supports_json
        && !json_requested
        && action.output_contract.as_deref() == Some("machine-readable")
    {
        command.push_str(" --json");
    }
}

fn action_label(action: VerificationCoreOpsActionKind) -> &'static str {
    match action {
        VerificationCoreOpsActionKind::Apply => "apply",
        VerificationCoreOpsActionKind::Explain => "explain",
        VerificationCoreOpsActionKind::Init => "init",
        VerificationCoreOpsActionKind::Plan => "plan",
        VerificationCoreOpsActionKind::Status => "status",
        VerificationCoreOpsActionKind::Agent => "agent",
    }
}

fn repository_source_arg<'a>(repository_source: &'a str, repo_fixture: &'a str) -> &'a str {
    if repository_source == "fixture" {
        repo_fixture
    } else {
        repository_source
    }
}

fn default_collection_status() -> VerificationArtifactCollectionStatus {
    VerificationArtifactCollectionStatus::Complete
}

fn validation_error(message: impl Into<String>) -> Result<(), CoreError> {
    Err(CoreError::new(FailureClass::Validation, message))
}

fn is_supported_assertion_type(value: &str) -> bool {
    matches!(
        value,
        "no_pending_changes"
            | "output_contains"
            | "step_command_contains"
            | "step_command_not_contains"
            | "step_stdout_contains"
            | "step_exit_code_is"
            | "step_duration_within_ms"
    )
}

fn assertion_requires_step_target(value: &str) -> bool {
    matches!(
        value,
        "step_command_contains"
            | "step_command_not_contains"
            | "step_stdout_contains"
            | "step_exit_code_is"
            | "step_duration_within_ms"
    )
}
