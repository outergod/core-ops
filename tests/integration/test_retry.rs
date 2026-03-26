use core_ops::core::errors::CoreError;
use core_ops::core::retry::{
    evaluate_retry_history, run_with_retry, RetryObservation, RetryPolicy,
};
use core_ops::core::types::{ConvergenceStatus, FailureClass};

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

#[test]
fn retry_detects_oscillation_across_repeated_attempts() {
    let history = vec![
        RetryObservation {
            attempt: 1,
            signature: "alpha.service:blocked:a".to_string(),
            affected_objects: vec!["alpha.service".to_string()],
        },
        RetryObservation {
            attempt: 2,
            signature: "alpha.service:blocked:b".to_string(),
            affected_objects: vec!["alpha.service".to_string()],
        },
        RetryObservation {
            attempt: 3,
            signature: "alpha.service:blocked:a".to_string(),
            affected_objects: vec!["alpha.service".to_string()],
        },
        RetryObservation {
            attempt: 4,
            signature: "alpha.service:blocked:b".to_string(),
            affected_objects: vec!["alpha.service".to_string()],
        },
    ];

    let evaluation = evaluate_retry_history(&history, 3).expect("evaluation");
    assert_eq!(evaluation.status, ConvergenceStatus::Oscillation);
}
