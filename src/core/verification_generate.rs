use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use crate::core::verification_model::{
    load_scenario_definition, VerificationCandidateReviewStatus, VerificationCandidateScenario,
    VerificationCoreOpsAction, VerificationCoreOpsActionKind, VerificationEnvironmentSelection,
    VerificationFixtureSet, VerificationScenarioClass, VerificationScenarioDefinition,
    VerificationScenarioSource, VerificationScenarioStep, VerificationStepTarget,
    VerificationStepType,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationInputs {
    pub observable_behaviors: Vec<String>,
    pub invariants: Vec<String>,
    pub idempotency_expectations: Vec<String>,
    pub failure_modes: Vec<String>,
    pub upgrade_considerations: Vec<String>,
    pub required_scenario_classes: Vec<VerificationScenarioClass>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationCoverageReport {
    pub required_classes: Vec<VerificationScenarioClass>,
    pub covered_classes: Vec<VerificationScenarioClass>,
    pub missing_classes: Vec<VerificationScenarioClass>,
}

pub fn extract_verification_inputs(spec_text: &str) -> Result<VerificationInputs, CoreError> {
    let guidance_present = spec_text.contains("## Verification Guidance")
        || spec_text.contains("## Verification Inputs");

    let mut observable_behaviors = collect_bullets(spec_text, "### Observable Behaviors");
    let invariants = collect_bullets(spec_text, "### Invariants");
    let idempotency_expectations = collect_bullets(spec_text, "### Idempotency Expectations");
    let failure_modes = collect_bullets(spec_text, "### Failure Modes");
    let upgrade_considerations = collect_bullets(spec_text, "### Upgrade Considerations");
    let required_scenario_classes = parse_explicit_classes(spec_text)?;

    if observable_behaviors.is_empty() {
        observable_behaviors = collect_requirement_behaviors(spec_text);
    }

    if observable_behaviors.is_empty() {
        return Err(CoreError::new(
            FailureClass::Validation,
            "feature spec does not expose enough semantic behavior to derive scenarios",
        ));
    }

    let derived_classes = if required_scenario_classes.is_empty() {
        infer_scenario_classes(&observable_behaviors)
    } else {
        required_scenario_classes
    };

    if derived_classes.is_empty() {
        return Err(CoreError::new(
            FailureClass::Validation,
            "feature spec does not expose enough scenario-class intent to derive candidates",
        ));
    }

    let mut generation_inputs = VerificationInputs {
        observable_behaviors,
        invariants,
        idempotency_expectations,
        failure_modes,
        upgrade_considerations,
        required_scenario_classes: derived_classes,
    };

    if !guidance_present && generation_inputs.invariants.is_empty() {
        generation_inputs
            .invariants
            .push("Derived from feature specification semantics".to_string());
    }

    Ok(generation_inputs)
}

pub fn load_accepted_corpus(dir: &Path) -> Result<Vec<VerificationScenarioDefinition>, CoreError> {
    let mut scenarios = Vec::new();
    for entry in fs::read_dir(dir).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read accepted corpus {}: {err}", dir.display()),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to read accepted corpus entry: {err}"),
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let scenario = load_scenario_definition(&path)?;
        if scenario.source == VerificationScenarioSource::Accepted {
            scenarios.push(scenario);
        }
    }
    Ok(scenarios)
}

pub fn generate_candidates_from_spec(
    spec_text: &str,
    accepted: &[VerificationScenarioDefinition],
) -> Result<Vec<VerificationCandidateScenario>, CoreError> {
    let inputs = extract_verification_inputs(spec_text)?;
    let behavioral_claim = inputs.observable_behaviors[0].clone();
    let scenario_classes = inputs.required_scenario_classes.clone();
    let primary_class = scenario_classes[0];

    let mut candidate = VerificationCandidateScenario {
        candidate_id: format!("candidate-{}", slugify(&behavioral_claim)),
        proposed_definition: VerificationScenarioDefinition {
            scenario_id: format!("candidate-{}", slugify(&behavioral_claim)),
            title: title_case(&behavioral_claim),
            description: behavioral_claim.clone(),
            scenario_classes: scenario_classes.clone(),
            source: VerificationScenarioSource::Candidate,
            behavioral_claim: behavioral_claim.clone(),
            rationale: inputs
                .invariants
                .first()
                .cloned()
                .unwrap_or_else(|| "Derived from feature specification semantics".to_string()),
            environment: VerificationEnvironmentSelection {
                profile: "single-blessed-vm".to_string(),
                overrides: None,
            },
            fixtures: VerificationFixtureSet {
                repo_fixture: "fixtures/repos/generated".to_string(),
                repository_evolution: None,
                config_inputs: Vec::new(),
                test_data: Vec::new(),
                revision_under_test: "candidate-revision".to_string(),
            },
            steps: vec![
                VerificationScenarioStep {
                    step_id: "boot".to_string(),
                    step_type: VerificationStepType::Boot,
                    target: VerificationStepTarget::Guest,
                    action: None,
                    command: None,
                    legacy_command_or_action: None,
                    expected_exit_behavior: None,
                    timeout_override: None,
                },
                VerificationScenarioStep {
                    step_id: "exercise-contract".to_string(),
                    step_type: VerificationStepType::CoreopsAction,
                    target: VerificationStepTarget::Guest,
                    action: Some(default_action_for_class(primary_class)),
                    command: None,
                    legacy_command_or_action: None,
                    expected_exit_behavior: None,
                    timeout_override: None,
                },
            ],
            assertions: vec![crate::core::verification_model::VerificationAssertionSpec {
                assertion_id: "derived-behavior".to_string(),
                assertion_type: assertion_type_for_class(primary_class).to_string(),
                target: "guest".to_string(),
                expected_state: expected_state_for_class(primary_class).to_string(),
                failure_message: format!("Derived behavior failed: {behavioral_claim}"),
                artifact_hints: inputs.failure_modes.clone(),
            }],
            policy_overrides: None,
        },
        generation_inputs: build_generation_inputs(&inputs),
        validation_findings: Vec::new(),
        review_status: VerificationCandidateReviewStatus::Generated,
    };

    if detect_duplicate_candidate(&candidate.proposed_definition, accepted) {
        candidate.mark_rejected("duplicate accepted coverage for normalized behavioral claim");
    } else {
        candidate.mark_needs_review();
    }

    Ok(vec![candidate])
}

pub fn detect_duplicate_candidate(
    candidate: &VerificationScenarioDefinition,
    accepted: &[VerificationScenarioDefinition],
) -> bool {
    accepted.iter().any(|accepted_scenario| {
        normalize_behavioral_claim(&accepted_scenario.behavioral_claim)
            == normalize_behavioral_claim(&candidate.behavioral_claim)
            && accepted_scenario
                .scenario_classes
                .iter()
                .any(|class| candidate.scenario_classes.contains(class))
    })
}

pub fn build_coverage_report(
    spec_text: &str,
    accepted: &[VerificationScenarioDefinition],
) -> Result<VerificationCoverageReport, CoreError> {
    let inputs = extract_verification_inputs(spec_text)?;
    let required = dedup_classes(inputs.required_scenario_classes);
    let covered = dedup_classes(
        accepted
            .iter()
            .flat_map(|scenario| scenario.scenario_classes.iter().copied())
            .collect(),
    );
    let missing = required
        .iter()
        .copied()
        .filter(|class| !covered.contains(class))
        .collect();

    Ok(VerificationCoverageReport {
        required_classes: required,
        covered_classes: covered,
        missing_classes: missing,
    })
}

pub fn normalize_behavioral_claim(value: &str) -> String {
    let normalized = value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>();
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render_candidate_yaml(
    candidate: &VerificationCandidateScenario,
) -> Result<String, CoreError> {
    serde_yaml::to_string(candidate).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to render candidate yaml: {err}"),
        )
    })
}

fn build_generation_inputs(inputs: &VerificationInputs) -> Vec<String> {
    let mut values = Vec::new();
    values.extend(inputs.observable_behaviors.clone());
    values.extend(inputs.invariants.clone());
    values.extend(inputs.failure_modes.clone());
    values
}

fn collect_bullets(spec_text: &str, heading: &str) -> Vec<String> {
    let mut lines = spec_text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == heading {
            let mut items = Vec::new();
            while let Some(next) = lines.peek() {
                let trimmed = next.trim();
                if trimmed.starts_with("### ") || trimmed.starts_with("## ") {
                    break;
                }
                if let Some(rest) = trimmed.strip_prefix("- ") {
                    items.push(rest.trim().to_string());
                }
                lines.next();
            }
            return items;
        }
    }
    Vec::new()
}

fn collect_requirement_behaviors(spec_text: &str) -> Vec<String> {
    let mut behaviors = collect_bullets(spec_text, "### Functional Requirements");
    if behaviors.is_empty() {
        behaviors = collect_bullets(spec_text, "## Functional Requirements");
    }
    if behaviors.is_empty() {
        behaviors = spec_text
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("- ")
                    .filter(|value| value.contains("SHALL") || value.contains("should"))
                    .map(|value| value.to_string())
            })
            .collect();
    }
    behaviors
}

fn parse_explicit_classes(spec_text: &str) -> Result<Vec<VerificationScenarioClass>, CoreError> {
    let candidates = collect_bullets(spec_text, "### Required Scenario Classes");
    candidates
        .into_iter()
        .map(|value| parse_scenario_class(&value))
        .collect()
}

fn infer_scenario_classes(observable_behaviors: &[String]) -> Vec<VerificationScenarioClass> {
    let mut classes = BTreeSet::new();
    for behavior in observable_behaviors {
        let normalized = normalize_behavioral_claim(behavior);
        if normalized.contains("idempot") || normalized.contains("reapply") {
            classes.insert(VerificationScenarioClass::Idempotency);
        }
        if normalized.contains("upgrade") || normalized.contains("transition") {
            classes.insert(VerificationScenarioClass::UpgradeTransition);
        }
        if normalized.contains("regression") {
            classes.insert(VerificationScenarioClass::RegressionDetection);
        }
        if normalized.contains("explain") {
            classes.insert(VerificationScenarioClass::ExplainApplyConsistency);
        }
    }
    if classes.is_empty() {
        classes.insert(VerificationScenarioClass::Convergence);
    }
    classes.into_iter().collect()
}

fn parse_scenario_class(value: &str) -> Result<VerificationScenarioClass, CoreError> {
    match value.trim() {
        "convergence" => Ok(VerificationScenarioClass::Convergence),
        "drift_correction" => Ok(VerificationScenarioClass::DriftCorrection),
        "idempotency" => Ok(VerificationScenarioClass::Idempotency),
        "upgrade_transition" => Ok(VerificationScenarioClass::UpgradeTransition),
        "reboot_resilience" => Ok(VerificationScenarioClass::RebootResilience),
        "explain_apply_consistency" => Ok(VerificationScenarioClass::ExplainApplyConsistency),
        "regression_detection" => Ok(VerificationScenarioClass::RegressionDetection),
        other => Err(CoreError::new(
            FailureClass::Validation,
            format!("unsupported required scenario class `{other}`"),
        )),
    }
}

fn default_action_for_class(class: VerificationScenarioClass) -> VerificationCoreOpsAction {
    let action = match class {
        VerificationScenarioClass::ExplainApplyConsistency => VerificationCoreOpsActionKind::Explain,
        VerificationScenarioClass::RegressionDetection => VerificationCoreOpsActionKind::Plan,
        VerificationScenarioClass::Idempotency
        | VerificationScenarioClass::UpgradeTransition
        | VerificationScenarioClass::Convergence
        | VerificationScenarioClass::DriftCorrection
        | VerificationScenarioClass::RebootResilience => VerificationCoreOpsActionKind::Apply,
    };

    VerificationCoreOpsAction {
        action,
        repository_source: "fixture".to_string(),
        revision: "candidate-revision".to_string(),
        object: None,
        host: None,
        mode: None,
        output_contract: None,
    }
}

fn assertion_type_for_class(class: VerificationScenarioClass) -> &'static str {
    match class {
        VerificationScenarioClass::Idempotency => "no_pending_changes",
        VerificationScenarioClass::UpgradeTransition => "output_contains",
        VerificationScenarioClass::Convergence => "output_contains",
        VerificationScenarioClass::DriftCorrection => "output_contains",
        VerificationScenarioClass::RebootResilience => "output_contains",
        VerificationScenarioClass::ExplainApplyConsistency => "output_contains",
        VerificationScenarioClass::RegressionDetection => "output_contains",
    }
}

fn expected_state_for_class(class: VerificationScenarioClass) -> &'static str {
    match class {
        VerificationScenarioClass::Idempotency => "none",
        VerificationScenarioClass::UpgradeTransition => "deterministic-transition",
        VerificationScenarioClass::Convergence => "converged",
        VerificationScenarioClass::DriftCorrection => "corrected",
        VerificationScenarioClass::RebootResilience => "recovered",
        VerificationScenarioClass::ExplainApplyConsistency => "consistent",
        VerificationScenarioClass::RegressionDetection => "stable",
    }
}

fn dedup_classes(classes: Vec<VerificationScenarioClass>) -> Vec<VerificationScenarioClass> {
    let mut by_name = BTreeMap::new();
    for class in classes {
        by_name.insert(format!("{class:?}"), class);
    }
    by_name.into_values().collect()
}

fn slugify(value: &str) -> String {
    normalize_behavioral_claim(value).replace(' ', "-")
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
