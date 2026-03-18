use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

#[test]
fn plan_does_not_apply_changes() {
    let repo = temp_dir("core_ops_repo_plan");
    let quadlets = repo.join("quadlets");
    fs::create_dir_all(&quadlets).expect("create quadlets");

    fs::write(
        quadlets.join("alpha.container"),
        "[Container]\nImage=alpine",
    )
    .expect("write quadlet");

    let host_quadlets = temp_dir("core_ops_host_plan");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(&repo, "rev-1").map_err(map_io_error),
        read_observed: &|| read_observed_state(&host_quadlets, Some("obs".to_string()))
            .map_err(map_io_error),
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &host_quadlets)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };

    let result = reconcile_plan(&deps).expect("plan");

    assert_eq!(result.run.summary, "planned");
    assert!(result.plan.actions.len() >= 1);
    assert!(fs::read_dir(&host_quadlets).unwrap().next().is_none());
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Plan,
        message: err.to_string(),
    }
}
