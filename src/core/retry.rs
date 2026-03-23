use crate::core::errors::CoreError;
use crate::core::types::FailureClass;

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
