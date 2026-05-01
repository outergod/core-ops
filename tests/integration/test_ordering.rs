use std::fs;
use std::path::PathBuf;

use core_ops::core::errors::CoreError;
use core_ops::core::reconcile::{reconcile_plan, ReconcileDependencies};
use core_ops::io::apply::apply_plan;
use core_ops::io::observed::read_observed_state;
use core_ops::io::repo::{load_desired_state, HOST_OVERRIDE_ENV};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{
    git_init_commit, materialize_skeleton, write_host_yaml, HostGuard,
};

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

#[test]
fn plan_orders_volume_before_container_before_socket() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Single service holding three artifacts. quadlet/ accepts container
    // and volume; systemd/ accepts socket. The planner orders globally
    // (volume → container → socket), not per-service, so a single
    // service exercises the same code path as three separate ones.
    let svc_quadlet = services.join("triad/quadlet");
    let svc_systemd = services.join("triad/systemd");
    fs::create_dir_all(&svc_quadlet).expect("create quadlet");
    fs::create_dir_all(&svc_systemd).expect("create systemd");
    fs::write(
        svc_quadlet.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("write container");
    fs::write(
        svc_systemd.join("beta.socket"),
        "[Socket]\nListenStream=8080\n",
    )
    .expect("write socket");
    fs::write(
        svc_quadlet.join("gamma.volume"),
        "[Volume]\nDriver=local\n",
    )
    .expect("write volume");
    write_host_yaml(&hosts, "example-host", &["triad"]);
    let rev = git_init_commit(tmp.path());

    let host_quadlets = temp_dir("core_ops_host_ordering");
    fs::create_dir_all(&host_quadlets).expect("create host quadlets");

    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");

    let repo_str = tmp.path().to_str().expect("utf-8 path");
    let deps = ReconcileDependencies {
        load_desired: &|| load_desired_state(repo_str, &rev).map_err(map_io_error),
        read_observed: &|desired| {
            read_observed_state(&host_quadlets, Some(desired), Some("obs".to_string()))
                .map_err(map_io_error)
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
