use crate::core::errors::CoreError;
use crate::core::retry::{run_with_retry as core_retry, RetryOutcome, RetryPolicy};
use crate::cli::diagnostics::{report_from_error, report_from_retry};
use miette::Report;

pub fn report_error(err: CoreError) -> Report {
    report_from_error(err)
}

pub fn report_retry_error(err: CoreError, attempts: usize, retryable: bool) -> Report {
    report_from_retry(err, attempts, retryable)
}

pub fn run_with_retry<T, F>(policy: &RetryPolicy, op: F) -> RetryOutcome<T>
where
    F: FnMut(usize) -> Result<T, CoreError>,
{
    core_retry(policy, op)
}
