//! Per-example integration test for `examples/05-observability/` (T020).

use std::path::Path;
use std::process::Command;

use core_ops::io::repo::{load_desired_state_from_path, HOST_OVERRIDE_ENV};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::HostGuard;

#[test]
fn example_05_observability_parses_and_plans() {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/05-observability");

    assert!(
        example_dir.join("README.md").exists(),
        "example root must carry README.md (FR-002)"
    );

    {
        let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
        let _host_guard = HostGuard::capture();
        std::env::set_var(HOST_OVERRIDE_ENV, "example");
        let desired = load_desired_state_from_path(
            &example_dir,
            example_dir.to_str().expect("utf-8 path"),
            "(stateless)",
        )
        .expect("parser load succeeds for 05-observability");
        let unit_names: Vec<String> = desired
            .workloads
            .iter()
            .map(|w| w.systemd_unit_name.clone())
            .collect();
        for expected in [
            "prometheus.container",
            "grafana.container",
            "node-exporter.container",
            "cadvisor.container",
        ] {
            assert!(
                unit_names.iter().any(|n| n == expected),
                "expected {expected} in {unit_names:?}"
            );
        }
    }

    let quadlet_dir = tempfile::TempDir::new().expect("tempdir");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops"))
        .arg("plan")
        .arg("--source-repo")
        .arg(&example_dir)
        .arg("--host")
        .arg("example")
        .arg("--quadlet-dir")
        .arg(quadlet_dir.path())
        .output()
        .expect("invoke core-ops binary");
    assert!(
        output.status.success(),
        "`core-ops plan --source-repo {} --host example` exited non-zero.\nstderr:\n{}",
        example_dir.display(),
        String::from_utf8_lossy(&output.stderr),
    );
}
