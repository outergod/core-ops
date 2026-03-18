use crate::cli::errors::{report_from_error, report_from_retry};
use crate::core::errors::CoreError;
use crate::core::retry::{run_with_retry as core_retry, RetryOutcome, RetryPolicy};

pub fn render_error(err: &CoreError) -> String {
    report_from_error(err).render()
}

pub fn render_retry_error(err: &CoreError, attempts: usize, retryable: bool) -> String {
    report_from_retry(err, attempts, retryable).render()
}

pub fn run_with_retry<T, F>(policy: &RetryPolicy, op: F) -> RetryOutcome<T>
where
    F: FnMut(usize) -> Result<T, CoreError>,
{
    core_retry(policy, op)
}
