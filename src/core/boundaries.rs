use crate::core::errors::CoreError;
use crate::core::types::{FailureClass, PlanAction, PlanActionType, ReconciliationPlan};
use crate::core::verification_model::{
    GuestCommandOutput, LibvirtGuestHandle, VerificationArtifactBundle,
    VerificationReadinessAcquisition, VerificationRunArtifacts, VerificationScenarioDefinition,
};
use std::path::Path;

pub fn enforce_plan_boundaries(plan: &ReconciliationPlan) -> Result<(), CoreError> {
    for action in &plan.actions {
        validate_action(action)?;
    }
    Ok(())
}

fn validate_action(action: &PlanAction) -> Result<(), CoreError> {
    if is_supported_action(&action.action_type) {
        return Ok(());
    }

    Err(CoreError::new(
        FailureClass::Validation,
        format!("unsupported mutation action: {:?}", action.action_type),
    ))
}

fn is_supported_action(action_type: &PlanActionType) -> bool {
    matches!(
        action_type,
        PlanActionType::PreparePath
            | PlanActionType::WriteQuadlet
            | PlanActionType::RemoveQuadlet
            | PlanActionType::ReloadSystemd
            | PlanActionType::StartUnit
            | PlanActionType::RestartUnit
            | PlanActionType::StopUnit
    )
}

pub trait VerificationLibvirtBoundary {
    fn create_guest(
        &self,
        scenario: &VerificationScenarioDefinition,
        workspace_root: &Path,
    ) -> Result<LibvirtGuestHandle, CoreError>;

    fn acquire_guest_readiness(
        &self,
        scenario: &VerificationScenarioDefinition,
        guest: &LibvirtGuestHandle,
    ) -> Result<VerificationReadinessAcquisition, CoreError>;

    fn destroy_guest(&self, guest: &LibvirtGuestHandle) -> Result<(), CoreError>;
}

pub trait VerificationGuestBoundary {
    fn wait_ready(
        &self,
        guest: &LibvirtGuestHandle,
        timeout: &str,
    ) -> Result<GuestCommandOutput, CoreError>;

    fn run_command(
        &self,
        guest: &LibvirtGuestHandle,
        command: &str,
        timeout: Option<&str>,
    ) -> Result<GuestCommandOutput, CoreError>;

    fn copy_to_guest(
        &self,
        guest: &LibvirtGuestHandle,
        local_path: &Path,
        remote_path: &str,
        recursive: bool,
        executable: bool,
    ) -> Result<(), CoreError>;
}

pub trait VerificationArtifactBoundary {
    fn collect_artifacts(
        &self,
        scenario: &VerificationScenarioDefinition,
        workspace_root: &Path,
    ) -> Result<VerificationRunArtifacts, CoreError>;

    fn write_bundle_manifest(&self, bundle: &VerificationArtifactBundle) -> Result<(), CoreError>;
}
