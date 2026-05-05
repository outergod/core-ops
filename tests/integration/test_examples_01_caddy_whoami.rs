//! Per-example integration test for `examples/01-caddy-whoami/` (T016).
//!
//! Asserts:
//! - (a) parser load via `load_desired_state_from_path` succeeds.
//! - (b) resolved service catalog contains the expected unit names.
//! - (c) example root carries `README.md`.
//! - (d) `core-ops plan --source-repo examples/01-caddy-whoami --host example`
//!   exits 0 (US1 AC1, SC-001/SC-003).

use std::path::Path;
use std::process::Command;

use core_ops::io::repo::{load_desired_state_from_path, HOST_OVERRIDE_ENV};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::HostGuard;

#[test]
fn example_01_caddy_whoami_parses_and_plans() {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/01-caddy-whoami");

    // (c) README at example root.
    assert!(
        example_dir.join("README.md").exists(),
        "example root must carry README.md (FR-002)"
    );

    // (a) parser load succeeds + (b) catalog contains expected services.
    {
        let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
        let _host_guard = HostGuard::capture();
        std::env::set_var(HOST_OVERRIDE_ENV, "example");
        let desired = load_desired_state_from_path(
            &example_dir,
            example_dir.to_str().expect("utf-8 path"),
            "(stateless)",
        )
        .expect("parser load succeeds for 01-caddy-whoami");
        let unit_names: Vec<String> = desired
            .workloads
            .iter()
            .map(|w| w.systemd_unit_name.clone())
            .collect();
        for expected in ["caddy.container", "whoami.container"] {
            assert!(
                unit_names.iter().any(|n| n == expected),
                "expected {expected} in {unit_names:?}"
            );
        }
    }

    // (d) `core-ops plan --source-repo <dir> --host example` exits 0.
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
        "`core-ops plan --source-repo {} --host example` exited non-zero.\nstdout:\n{}\nstderr:\n{}",
        example_dir.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        !output.stdout.is_empty(),
        "plan output should be non-empty against a fresh quadlet dir"
    );
}
