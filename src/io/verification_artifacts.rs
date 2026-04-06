use crate::core::boundaries::VerificationArtifactBoundary;
use crate::core::errors::CoreError;
use crate::core::types::{
    FailureClass, VerificationArtifactCollectionStatus, VerificationRunOutcome,
};
use crate::core::verification_model::{
    VerificationArtifactBundle, VerificationArtifactManifestEntry, VerificationAssertionResult,
    VerificationRunArtifacts, VerificationScenarioClass, VerificationScenarioDefinition,
    VerificationStepResult,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn write_artifact_manifest(bundle: &VerificationArtifactBundle) -> Result<(), CoreError> {
    let manifest_path = Path::new(&bundle.bundle_path).join("manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to create artifact manifest directory: {err}"),
            )
        })?;
    }
    let payload = serde_json::to_string_pretty(bundle).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!("failed to serialize artifact manifest: {err}"),
        )
    })?;
    fs::write(&manifest_path, payload).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to write artifact manifest {}: {err}",
                manifest_path.display()
            ),
        )
    })
}

pub fn build_run_artifacts(
    bundle_path: impl Into<String>,
    required_entries: Vec<VerificationArtifactManifestEntry>,
    environment_retained: bool,
) -> VerificationRunArtifacts {
    VerificationRunArtifacts {
        bundle: VerificationArtifactBundle {
            bundle_path: bundle_path.into(),
            manifest_entries: required_entries.clone(),
            always_collected_entries: required_entries
                .iter()
                .filter(|entry| entry.required)
                .map(|entry| entry.logical_name.clone())
                .collect(),
            failure_specific_entries: Vec::new(),
            environment_retained,
            collection_status: VerificationArtifactCollectionStatus::Complete,
        },
        warnings: Vec::new(),
    }
}

pub struct VerificationArtifactEnrichment {
    pub regression_summary: Option<String>,
    pub promotion_status: Option<String>,
}

pub fn write_diagnostic_artifacts(
    bundle_root: &Path,
    scenario: &VerificationScenarioDefinition,
    overall_outcome: VerificationRunOutcome,
    failure_summary: Option<&str>,
    step_results: &[VerificationStepResult],
    assertion_results: &[VerificationAssertionResult],
) -> Result<VerificationArtifactEnrichment, CoreError> {
    fs::create_dir_all(bundle_root).map_err(|err| {
        CoreError::new(
            FailureClass::Apply,
            format!(
                "failed to create artifact bundle root {}: {err}",
                bundle_root.display()
            ),
        )
    })?;

    let regression_summary = render_regression_summary(scenario);
    if let Some(summary) = &regression_summary {
        fs::write(bundle_root.join("regression-summary.txt"), summary).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write regression summary artifact: {err}"),
            )
        })?;
    }

    let promotion_status = render_promotion_status(scenario);
    if let Some(status) = &promotion_status {
        fs::write(bundle_root.join("promotion-status.txt"), status).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write promotion status artifact: {err}"),
            )
        })?;
    }

    if overall_outcome != VerificationRunOutcome::Passed {
        let failed_step = step_results
            .iter()
            .find(|step| step.status != crate::core::types::VerificationStepStatus::Passed);
        let payload = render_failure_summary(
            scenario,
            overall_outcome,
            failure_summary,
            failed_step,
            assertion_results,
        );
        fs::write(bundle_root.join("failure-summary.txt"), payload).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!("failed to write failure summary artifact: {err}"),
            )
        })?;
    }

    Ok(VerificationArtifactEnrichment {
        regression_summary,
        promotion_status,
    })
}

#[derive(Clone, Debug, Default)]
pub struct ArtifactCollector;

impl VerificationArtifactBoundary for ArtifactCollector {
    fn collect_artifacts(
        &self,
        scenario: &VerificationScenarioDefinition,
        workspace_root: &Path,
    ) -> Result<VerificationRunArtifacts, CoreError> {
        let artifact_policy = scenario.effective_artifact_policy()?;
        let bundle_root = workspace_root.join("artifacts");
        fs::create_dir_all(&bundle_root).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!(
                    "failed to create artifact bundle root {}: {err}",
                    bundle_root.display()
                ),
            )
        })?;

        let mut warnings = Vec::new();
        let mut manifest_entries = Vec::new();
        for logical_name in &artifact_policy.always_collect {
            let relative_path = format!("{logical_name}.txt");
            if logical_name == "force-fail-artifact" {
                warnings.push("artifact collection skipped force-fail-artifact".to_string());
                manifest_entries.push(VerificationArtifactManifestEntry {
                    logical_name: logical_name.clone(),
                    relative_path,
                    required: true,
                });
                continue;
            }
            let full_path = bundle_root.join(&relative_path);
            if !full_path.exists() {
                fs::write(
                    &full_path,
                    format!("artifact {logical_name} for {}", scenario.scenario_id),
                )
                .map_err(|err| {
                    CoreError::new(
                        FailureClass::Apply,
                        format!("failed to write artifact {}: {err}", full_path.display()),
                    )
                })?;
            }
            manifest_entries.push(VerificationArtifactManifestEntry {
                logical_name: logical_name.clone(),
                relative_path,
                required: true,
            });
        }

        let static_network_config = bundle_root.join("static-network-config.txt");
        if static_network_config.exists() {
            manifest_entries.push(VerificationArtifactManifestEntry {
                logical_name: "static-network-config".to_string(),
                relative_path: "static-network-config.txt".to_string(),
                required: false,
            });
        }
        let qemu_launch_log = bundle_root.join("qemu-launch-log.txt");
        if qemu_launch_log.exists() {
            manifest_entries.push(VerificationArtifactManifestEntry {
                logical_name: "qemu-launch-log".to_string(),
                relative_path: "qemu-launch-log.txt".to_string(),
                required: false,
            });
        }
        let rendered_butane = bundle_root.join("rendered-ignition.bu");
        if rendered_butane.exists() {
            manifest_entries.push(VerificationArtifactManifestEntry {
                logical_name: "rendered-ignition-butane".to_string(),
                relative_path: "rendered-ignition.bu".to_string(),
                required: false,
            });
        }
        let rendered_ignition = bundle_root.join("rendered-ignition.ign");
        if rendered_ignition.exists() {
            manifest_entries.push(VerificationArtifactManifestEntry {
                logical_name: "rendered-ignition".to_string(),
                relative_path: "rendered-ignition.ign".to_string(),
                required: false,
            });
        }
        for (logical_name, relative_path) in [
            ("guest-journal", "guest-journal.txt"),
            ("systemctl-failed", "systemctl-failed.txt"),
            ("coreops-state", "coreops-status.json"),
            ("quadlet-dir-list", "quadlet-dir-list.txt"),
            ("systemd-dir-list", "systemd-dir-list.txt"),
            ("failure-summary", "failure-summary.txt"),
            ("regression-summary", "regression-summary.txt"),
            ("promotion-status", "promotion-status.txt"),
        ] {
            if bundle_root.join(relative_path).exists() {
                manifest_entries.push(VerificationArtifactManifestEntry {
                    logical_name: logical_name.to_string(),
                    relative_path: relative_path.to_string(),
                    required: false,
                });
            }
        }
        for entry in fs::read_dir(&bundle_root).map_err(|err| {
            CoreError::new(
                FailureClass::Apply,
                format!(
                    "failed to read artifact bundle root {}: {err}",
                    bundle_root.display()
                ),
            )
        })? {
            let entry = entry.map_err(|err| {
                CoreError::new(
                    FailureClass::Apply,
                    format!("failed to inspect artifact bundle entry: {err}"),
                )
            })?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with("unit-status-") || file_name.starts_with("unit-journal-") {
                manifest_entries.push(VerificationArtifactManifestEntry {
                    logical_name: file_name.trim_end_matches(".txt").to_string(),
                    relative_path: file_name.to_string(),
                    required: false,
                });
            }
        }

        let collection_status = if warnings.is_empty() {
            VerificationArtifactCollectionStatus::Complete
        } else {
            VerificationArtifactCollectionStatus::Partial
        };
        let manifest_names = manifest_entries
            .iter()
            .map(|entry| entry.logical_name.as_str())
            .collect::<BTreeSet<_>>();
        let mut failure_specific_entries = artifact_policy
            .collect_on_failure
            .iter()
            .filter(|entry| manifest_names.contains(entry.as_str()))
            .cloned()
            .collect::<BTreeSet<_>>();
        for logical_name in [
            "failure-summary",
            "guest-journal",
            "systemctl-failed",
            "coreops-state",
            "quadlet-dir-list",
            "systemd-dir-list",
            "regression-summary",
        ] {
            if manifest_names.contains(logical_name) {
                failure_specific_entries.insert(logical_name.to_string());
            }
        }
        Ok(VerificationRunArtifacts {
            bundle: VerificationArtifactBundle {
                bundle_path: bundle_root.display().to_string(),
                manifest_entries: manifest_entries.clone(),
                always_collected_entries: manifest_entries
                    .iter()
                    .filter(|entry| entry.required)
                    .map(|entry| entry.logical_name.clone())
                    .collect(),
                failure_specific_entries: failure_specific_entries.into_iter().collect(),
                environment_retained: false,
                collection_status,
            },
            warnings,
        })
    }

    fn write_bundle_manifest(&self, bundle: &VerificationArtifactBundle) -> Result<(), CoreError> {
        write_artifact_manifest(bundle)
    }
}

fn render_regression_summary(scenario: &VerificationScenarioDefinition) -> Option<String> {
    let evolution = scenario.fixtures.repository_evolution.as_ref()?;
    let revision_sequence = if evolution.revisions.is_empty() {
        scenario.fixtures.revision_under_test.clone()
    } else {
        evolution.revisions.join(" -> ")
    };
    let states = if evolution.states.is_empty() {
        "unspecified".to_string()
    } else {
        evolution.states.join(", ")
    };
    let expectations = if evolution.transition_expectations.is_empty() {
        "unspecified".to_string()
    } else {
        evolution.transition_expectations.join(", ")
    };
    let history_fixture = evolution
        .history_fixture
        .clone()
        .unwrap_or_else(|| "unspecified".to_string());
    Some(format!(
        "Scenario: {}\nRevision under test: {}\nRevision sequence: {}\nHistory fixture: {}\nStates: {}\nTransition expectations: {}\n",
        scenario.scenario_id,
        scenario.fixtures.revision_under_test,
        revision_sequence,
        history_fixture,
        states,
        expectations
    ))
}

fn render_promotion_status(scenario: &VerificationScenarioDefinition) -> Option<String> {
    if scenario.source == crate::core::verification_model::VerificationScenarioSource::Accepted
        && scenario
            .scenario_classes
            .contains(&VerificationScenarioClass::RegressionDetection)
    {
        Some(
            "accepted permanent regression scenario derived from a bug reproduction".to_string(),
        )
    } else {
        None
    }
}

fn render_failure_summary(
    scenario: &VerificationScenarioDefinition,
    outcome: VerificationRunOutcome,
    failure_summary: Option<&str>,
    failed_step: Option<&VerificationStepResult>,
    assertion_results: &[VerificationAssertionResult],
) -> String {
    let mut payload = String::new();
    payload.push_str(&format!("Scenario: {}\n", scenario.scenario_id));
    payload.push_str(&format!("Outcome: {:?}\n", outcome));
    if let Some(summary) = failure_summary {
        payload.push_str(&format!("Summary: {}\n", summary));
    }
    if let Some(step) = failed_step {
        payload.push_str(&format!("Failed Step: {}\n", step.step_id));
        if let Some(command) = &step.command {
            payload.push_str(&format!("Command: {}\n", command));
        }
        if let Some(exit_code) = step.exit_code {
            payload.push_str(&format!("Exit Code: {}\n", exit_code));
        }
    }
    let failed_assertions = assertion_results
        .iter()
        .filter(|result| result.status != crate::core::types::VerificationAssertionStatus::Passed)
        .map(|result| result.assertion_id.as_str())
        .collect::<Vec<_>>();
    if !failed_assertions.is_empty() {
        payload.push_str(&format!(
            "Failed Assertions: {}\n",
            failed_assertions.join(", ")
        ));
    }
    payload
}
