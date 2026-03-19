use std::fs;
use std::path::PathBuf;

use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::load_desired_state;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

fn init_git_repo(repo: &PathBuf) -> String {
    std::process::Command::new("git")
        .arg("init")
        .arg(repo)
        .output()
        .expect("git init");

    let quadlets = repo.join("quadlets");
    fs::create_dir_all(&quadlets).expect("create quadlets");
    fs::write(quadlets.join("alpha.container"), "[Container]\nImage=alpine")
        .expect("write container");
    fs::write(quadlets.join("beta.socket"), "[Socket]\nListenStream=8080")
        .expect("write socket");
    fs::write(quadlets.join("gamma.volume"), "[Volume]\nDriver=local")
        .expect("write volume");

    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("add")
        .arg(".")
        .output()
        .expect("git add");
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("commit")
        .arg("-m")
        .arg("fixture")
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .output()
        .expect("git commit");

    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .expect("git rev-parse");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn plan_orders_volume_before_container_before_socket() {
    let repo = temp_dir("core_ops_repo_ordering");
    let rev = init_git_repo(&repo);

    let host_quadlets = temp_dir("core_ops_host_ordering");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo.to_str().unwrap(), &rev).map_err(map_io_error),
        read_observed: &|| {
            read_observed_state(&host_quadlets, Some("obs".to_string())).map_err(map_io_error)
        },
        apply_plan: &|plan, desired| {
            apply_plan(plan, &desired.workloads, &host_quadlets, false)
                .map(|_| ())
                .map_err(map_io_error)
        },
    };

    let result = reconcile_plan(&deps).expect("plan");
    let mut ordered_targets = Vec::new();
    for action in result.plan.actions {
        if ordered_targets.last() != Some(&action.target) {
            ordered_targets.push(action.target);
        }
    }

    assert_eq!(
        ordered_targets,
        vec![
            "gamma.volume".to_string(),
            "alpha.container".to_string(),
            "beta.socket".to_string(),
        ]
    );
}

fn map_io_error<E: std::fmt::Display>(err: E) -> CoreError {
    CoreError {
        class: core_ops::core::types::FailureClass::Plan,
        message: err.to_string(),
    }
}
