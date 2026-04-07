use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use crate::core::types::{
    VerificationAssertionStatus, VerificationRunMode, VerificationRunOutcome,
    VerificationStepStatus,
};
use crate::core::verification_model::{
    VerificationAssertionResult, VerificationAssertionSpec, VerificationExecutionPlan,
    VerificationPlannedStep, VerificationReadinessExpectation, VerificationReadinessRecord,
    VerificationReadinessRejection, VerificationReadinessRejectionKind, VerificationRuntimeBindings,
    VerificationScenarioDefinition, VerificationScenarioOutcome, VerificationStepResult,
    VerificationStepType,
};
use std::time::Duration;

pub fn build_execution_plan(
    scenario: &VerificationScenarioDefinition,
    run_id: impl Into<String>,
    mode: VerificationRunMode,
    bindings: Option<&VerificationRuntimeBindings>,
) -> Result<VerificationExecutionPlan, crate::core::errors::CoreError> {
    let timeouts = scenario.effective_timeouts()?;
    let step_sequence = scenario
        .steps
        .iter()
        .map(|step| {
            Ok(VerificationPlannedStep {
                step_id: step.step_id.clone(),
                step_type: step.step_type,
                target: step.target,
                effective_timeout: step
                    .timeout_override
                    .clone()
                    .unwrap_or_else(|| default_timeout_for_step(&timeouts, step.step_type)),
                command_or_action: scenario.render_step_command(step, bindings)?,
            })
        })
        .collect::<Result<Vec<_>, crate::core::errors::CoreError>>()?;

    Ok(VerificationExecutionPlan {
        scenario_id: scenario.scenario_id.clone(),
        run_id: run_id.into(),
        mode,
        step_sequence,
        retain_environment: should_retain_environment(mode, scenario),
    })
}

pub fn evaluate_assertion_results(
    results: &[VerificationAssertionResult],
) -> VerificationRunOutcome {
    if results
        .iter()
        .any(|result| result.status == VerificationAssertionStatus::TimedOut)
    {
        VerificationRunOutcome::Timeout
    } else if results
        .iter()
        .any(|result| result.status == VerificationAssertionStatus::Failed)
    {
        VerificationRunOutcome::AssertionFailure
    } else if results
        .iter()
        .all(|result| result.status == VerificationAssertionStatus::Passed)
    {
        VerificationRunOutcome::Passed
    } else {
        VerificationRunOutcome::HarnessError
    }
}

pub fn classify_scenario_outcome(
    step_results: &[VerificationStepResult],
    assertion_results: &[VerificationAssertionResult],
) -> VerificationRunOutcome {
    if step_results
        .iter()
        .any(|step| step.status == VerificationStepStatus::TimedOut)
    {
        VerificationRunOutcome::Timeout
    } else if step_results.iter().any(|step| {
        step.status == VerificationStepStatus::Failed
            && step.step_type == VerificationStepType::CoreopsAction
    }) {
        VerificationRunOutcome::AssertionFailure
    } else if step_results
        .iter()
        .any(|step| step.status == VerificationStepStatus::Failed)
    {
        VerificationRunOutcome::InfrastructureFailure
    } else {
        evaluate_assertion_results(assertion_results)
    }
}

pub fn classify_run_outcome(outcomes: &[VerificationScenarioOutcome]) -> VerificationRunOutcome {
    if outcomes.is_empty() {
        return VerificationRunOutcome::HarnessError;
    }

    if outcomes
        .iter()
        .all(|outcome| outcome.outcome == VerificationRunOutcome::Passed)
    {
        return VerificationRunOutcome::Passed;
    }

    for kind in [
        VerificationRunOutcome::InfrastructureFailure,
        VerificationRunOutcome::Timeout,
        VerificationRunOutcome::HarnessError,
        VerificationRunOutcome::AssertionFailure,
    ] {
        if outcomes.iter().any(|outcome| outcome.outcome == kind) {
            return kind;
        }
    }

    VerificationRunOutcome::HarnessError
}

pub fn evaluate_assertions(
    assertions: &[VerificationAssertionSpec],
    step_results: &[VerificationStepResult],
) -> Result<Vec<VerificationAssertionResult>, CoreError> {
    assertions
        .iter()
        .map(|assertion| {
            let (status, observed_value) = evaluate_assertion(assertion, step_results)?;
            Ok(VerificationAssertionResult {
                assertion_id: assertion.assertion_id.clone(),
                status,
                observed_value,
                evidence_refs: vec!["artifacts/assertions/latest.json".to_string()],
            })
        })
        .collect()
}

pub fn should_retain_environment(
    mode: VerificationRunMode,
    scenario: &VerificationScenarioDefinition,
) -> bool {
    matches!(mode, VerificationRunMode::Debug)
        && scenario
            .effective_artifact_policy()
            .map(|policy| policy.retain_environment_in_debug)
            .unwrap_or(false)
}

pub fn parse_timeout_literal(timeout: &str) -> Result<Duration, CoreError> {
    let trimmed = timeout.trim();
    let seconds = trimmed
        .strip_suffix('s')
        .unwrap_or(trimmed)
        .parse::<u64>()
        .map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid timeout `{timeout}`: {err}"),
            )
        })?;
    Ok(Duration::from_secs(seconds))
}

pub fn parse_readiness_record_line(
    line: &str,
    marker: &str,
) -> Result<VerificationReadinessRecord, VerificationReadinessRejection> {
    let prefix = format!("{marker} ");
    let payload = line.trim();
    let Some(marker_index) = payload.find(&prefix) else {
        return Err(VerificationReadinessRejection {
            kind: VerificationReadinessRejectionKind::Malformed,
            summary: format!("readiness line missing marker `{marker}`"),
            raw_line: Some(payload.to_string()),
        });
    };
    let json = &payload[(marker_index + prefix.len())..];
    let record = serde_json::from_str::<VerificationReadinessRecord>(json).map_err(|err| {
        VerificationReadinessRejection {
            kind: VerificationReadinessRejectionKind::Malformed,
            summary: format!("readiness record is not valid JSON: {err}"),
            raw_line: Some(payload.to_string()),
        }
    })?;
    Ok(record)
}

pub fn validate_readiness_record(
    record: &VerificationReadinessRecord,
    expectation: &VerificationReadinessExpectation,
    raw_line: &str,
) -> Result<(), VerificationReadinessRejection> {
    if record.run_id != expectation.run_id || record.token != expectation.token {
        return Err(VerificationReadinessRejection {
            kind: VerificationReadinessRejectionKind::Stale,
            summary: "readiness record does not match the current run identity".to_string(),
            raw_line: Some(raw_line.to_string()),
        });
    }
    if !is_usable_ipv4(&record.ip) {
        return Err(VerificationReadinessRejection {
            kind: VerificationReadinessRejectionKind::Malformed,
            summary: "readiness record does not contain a usable IPv4 address".to_string(),
            raw_line: Some(raw_line.to_string()),
        });
    }
    Ok(())
}

pub fn evaluate_readiness_line(
    line: &str,
    expectation: &VerificationReadinessExpectation,
) -> Result<VerificationReadinessRecord, VerificationReadinessRejection> {
    let record = parse_readiness_record_line(line, &expectation.marker)?;
    validate_readiness_record(&record, expectation, line)?;
    Ok(record)
}

pub fn accept_first_valid_readiness(
    accepted: &Option<VerificationReadinessRecord>,
    candidate: VerificationReadinessRecord,
    raw_line: &str,
) -> Result<VerificationReadinessRecord, VerificationReadinessRejection> {
    if accepted.is_some() {
        return Err(VerificationReadinessRejection {
            kind: VerificationReadinessRejectionKind::DuplicateCurrentRun,
            summary: "later current-run readiness record ignored after first acceptance".to_string(),
            raw_line: Some(raw_line.to_string()),
        });
    }
    Ok(candidate)
}

pub fn is_usable_ipv4(value: &str) -> bool {
    value
        .parse::<std::net::Ipv4Addr>()
        .map(|ip| !ip.is_unspecified() && !ip.is_multicast())
        .unwrap_or(false)
}

fn default_timeout_for_step(
    timeouts: &crate::core::verification_model::VerificationTimeoutPolicy,
    step_type: VerificationStepType,
) -> String {
    timeouts
        .per_step_defaults
        .get(step_type_key(step_type))
        .cloned()
        .unwrap_or_else(|| timeouts.readiness_timeout.clone())
}

fn step_type_key(step_type: VerificationStepType) -> &'static str {
    match step_type {
        VerificationStepType::Boot => "boot",
        VerificationStepType::WaitReady => "wait_ready",
        VerificationStepType::CoreopsAction => "coreops_action",
        VerificationStepType::GuestCommand => "guest_command",
        VerificationStepType::MutateState => "mutate_state",
        VerificationStepType::Reboot => "reboot",
    }
}

fn evaluate_assertion(
    assertion: &VerificationAssertionSpec,
    step_results: &[VerificationStepResult],
) -> Result<(VerificationAssertionStatus, Option<String>), CoreError> {
    let observed_value = last_stdout(step_results);
    let result = match assertion.assertion_type.as_str() {
        "no_pending_changes" => (
            if step_results.iter().any(|step| {
                step.stdout
                    .as_deref()
                    .map(assertion_matches_no_pending_changes)
                    .unwrap_or(false)
            }) {
                VerificationAssertionStatus::Passed
            } else {
                VerificationAssertionStatus::Failed
            },
            observed_value,
        ),
        "output_contains" => (
            if step_results.iter().any(|step| {
                step.stdout
                    .as_deref()
                    .map(|stdout| stdout.contains(&assertion.expected_state))
                    .unwrap_or(false)
            }) {
                VerificationAssertionStatus::Passed
            } else {
                VerificationAssertionStatus::Failed
            },
            observed_value,
        ),
        "step_command_contains" => evaluate_step_command_contains(assertion, step_results, true),
        "step_command_not_contains" => evaluate_step_command_contains(assertion, step_results, false),
        "step_stdout_contains" => {
            let step = find_target_step(assertion, step_results)?;
            let observed = step.stdout.clone();
            (
                if step
                    .stdout
                    .as_deref()
                    .map(|stdout| stdout.contains(&assertion.expected_state))
                    .unwrap_or(false)
                {
                    VerificationAssertionStatus::Passed
                } else {
                    VerificationAssertionStatus::Failed
                },
                observed,
            )
        }
        "step_exit_code_is" => {
            let step = find_target_step(assertion, step_results)?;
            let expected = assertion.expected_state.parse::<i32>().map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!(
                        "assertion `{}` expected_state must parse as exit code: {err}",
                        assertion.assertion_id
                    ),
                )
            })?;
            let observed = step.exit_code.map(|value| value.to_string());
            (
                if step.exit_code == Some(expected) {
                    VerificationAssertionStatus::Passed
                } else {
                    VerificationAssertionStatus::Failed
                },
                observed,
            )
        }
        "step_duration_within_ms" => {
            let step = find_target_step(assertion, step_results)?;
            let expected = assertion.expected_state.parse::<u64>().map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!(
                        "assertion `{}` expected_state must parse as milliseconds: {err}",
                        assertion.assertion_id
                    ),
                )
            })?;
            let observed = step.duration_ms.map(|value| value.to_string());
            (
                if step.duration_ms.map(|value| value <= expected).unwrap_or(false) {
                    VerificationAssertionStatus::Passed
                } else {
                    VerificationAssertionStatus::Failed
                },
                observed,
            )
        }
        _ => (
            VerificationAssertionStatus::Failed,
            Some(format!(
                "unsupported assertion type {}",
                assertion.assertion_type
            )),
        ),
    };
    Ok(result)
}

fn find_target_step<'a>(
    assertion: &VerificationAssertionSpec,
    step_results: &'a [VerificationStepResult],
) -> Result<&'a VerificationStepResult, CoreError> {
    step_results
        .iter()
        .find(|step| step.step_id == assertion.target)
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                format!(
                    "assertion `{}` references unknown step `{}`",
                    assertion.assertion_id, assertion.target
                ),
            )
        })
}

fn evaluate_step_command_contains(
    assertion: &VerificationAssertionSpec,
    step_results: &[VerificationStepResult],
    should_contain: bool,
) -> (VerificationAssertionStatus, Option<String>) {
    if let Some(step) = step_results
        .iter()
        .find(|step| step.step_id == assertion.target)
    {
        let observed = step.command.clone();
        let contains = step
            .command
            .as_deref()
            .map(|command| command.contains(&assertion.expected_state))
            .unwrap_or(false);
        (
            if contains == should_contain {
                VerificationAssertionStatus::Passed
            } else {
                VerificationAssertionStatus::Failed
            },
            observed,
        )
    } else {
        (
            VerificationAssertionStatus::Failed,
            Some(format!("missing step {}", assertion.target)),
        )
    }
}

fn last_stdout(step_results: &[VerificationStepResult]) -> Option<String> {
    step_results.iter().rev().find_map(|step| step.stdout.clone())
}

pub fn assertion_matches_no_pending_changes(output: &str) -> bool {
    let normalized = strip_ansi_escapes(output);
    normalized.contains("no managed changes")
        || (normalized.contains("unchanged") && normalized.contains("Outcome: converged"))
}

fn strip_ansi_escapes(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    let iter = chars.by_ref();
                    while let Some(code) = iter.next() {
                        if code == '\u{7}' {
                            break;
                        }
                        if code == '\u{1b}' && matches!(iter.peek(), Some('\\')) {
                            iter.next();
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            normalized.push(ch);
        }
    }
    normalized
}
