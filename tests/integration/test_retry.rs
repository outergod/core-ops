use core_ops::core::errors::CoreError;
use core_ops::core::retry::{run_with_retry, RetryPolicy};
use core_ops::core::types::FailureClass;

#[test]
fn retry_succeeds_after_transient_failures() {
    let policy = RetryPolicy::bounded(3);
    let mut attempts = 0;

    let outcome = run_with_retry(&policy, |_| {
        attempts += 1;
        if attempts < 3 {
            Err(CoreError::new(FailureClass::Transient, "temporary"))
        } else {
            Ok("ok")
        }
    });

    assert_eq!(outcome.attempts, 3);
    assert!(outcome.result.is_ok());
}
