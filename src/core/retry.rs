use crate::core::errors::CoreError;
use crate::core::types::{ConvergenceStatus, FailureClass, VerificationResult, VerificationStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: usize,
}

impl RetryPolicy {
    pub fn bounded(max_attempts: usize) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryOutcome<T> {
    pub result: Result<T, CoreError>,
    pub attempts: usize,
    pub retryable: bool,
}

pub fn run_with_retry<T, F>(policy: &RetryPolicy, mut op: F) -> RetryOutcome<T>
where
    F: FnMut(usize) -> Result<T, CoreError>,
{
    let mut attempts = 0;

    for attempt in 1..=policy.max_attempts {
        attempts = attempt;
        match op(attempt) {
            Ok(value) => {
                return RetryOutcome {
                    result: Ok(value),
                    attempts,
                    retryable: false,
                }
            }
            Err(err) => {
                let retryable = is_retryable_error(&err);
                if !retryable || attempt == policy.max_attempts {
                    return RetryOutcome {
                        result: Err(err),
                        attempts,
                        retryable,
                    };
                }
            }
        }
    }

    RetryOutcome {
        result: Err(CoreError::new(
            FailureClass::Transient,
            "retry policy exhausted without attempts",
        )),
        attempts,
        retryable: true,
    }
}

pub fn is_retryable_error(err: &CoreError) -> bool {
    err.class == FailureClass::Transient
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryObservation {
    pub attempt: u32,
    pub signature: String,
    pub affected_objects: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryEvaluation {
    pub status: ConvergenceStatus,
    pub attempt_count: u32,
    pub signature: String,
    pub affected_objects: Vec<String>,
}

pub fn build_retry_observation(
    attempt: u32,
    verification_results: &[VerificationResult],
) -> RetryObservation {
    let mut failures = verification_results
        .iter()
        .filter(|result| result.status == VerificationStatus::Failure)
        .map(|result| {
            (
                result.target.clone(),
                result
                    .details
                    .clone()
                    .unwrap_or_else(|| "failure".to_string()),
            )
        })
        .collect::<Vec<_>>();
    failures.sort();

    let signature = failures
        .iter()
        .map(|(target, details)| format!("{target}:{details}"))
        .collect::<Vec<_>>()
        .join("|");
    let affected_objects = failures
        .iter()
        .map(|(target, _)| target.clone())
        .collect::<Vec<_>>();

    RetryObservation {
        attempt,
        signature,
        affected_objects,
    }
}

pub fn evaluate_retry_history(
    history: &[RetryObservation],
    retry_budget: u32,
) -> Option<RetryEvaluation> {
    let latest = history.last()?;
    if latest.signature.is_empty() {
        return Some(RetryEvaluation {
            status: ConvergenceStatus::Success,
            attempt_count: latest.attempt,
            signature: latest.signature.clone(),
            affected_objects: latest.affected_objects.clone(),
        });
    }

    if let Some(status) = detect_oscillation(history) {
        return Some(RetryEvaluation {
            status,
            attempt_count: latest.attempt,
            signature: latest.signature.clone(),
            affected_objects: latest.affected_objects.clone(),
        });
    }

    if latest.signature.contains("blocked:") {
        return Some(RetryEvaluation {
            status: ConvergenceStatus::Blocked,
            attempt_count: latest.attempt,
            signature: latest.signature.clone(),
            affected_objects: latest.affected_objects.clone(),
        });
    }

    let repeated_count = history
        .iter()
        .rev()
        .take_while(|entry| entry.signature == latest.signature)
        .count() as u32;
    if repeated_count >= retry_budget {
        return Some(RetryEvaluation {
            status: ConvergenceStatus::RepeatedFailure,
            attempt_count: latest.attempt,
            signature: latest.signature.clone(),
            affected_objects: latest.affected_objects.clone(),
        });
    }

    Some(RetryEvaluation {
        status: ConvergenceStatus::Failed,
        attempt_count: latest.attempt,
        signature: latest.signature.clone(),
        affected_objects: latest.affected_objects.clone(),
    })
}

fn detect_oscillation(history: &[RetryObservation]) -> Option<ConvergenceStatus> {
    if history.len() < 4 {
        return None;
    }
    let a = &history[history.len() - 1].signature;
    let b = &history[history.len() - 2].signature;
    let c = &history[history.len() - 3].signature;
    let d = &history[history.len() - 4].signature;

    if !a.is_empty() && a == c && b == d && a != b {
        Some(ConvergenceStatus::Oscillation)
    } else {
        None
    }
}
