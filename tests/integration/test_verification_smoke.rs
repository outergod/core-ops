use std::process::Command;

fn fixture_path(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn vm_backed_verification_smoke_runs_against_real_vm_when_enabled() {
    if std::env::var("CORE_OPS_VERIFY_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skipping VM-backed verification smoke test; set CORE_OPS_VERIFY_SMOKE=1");
        return;
    }

    let workspace = tempfile::tempdir().expect("workspace");
    let artifacts = tempfile::tempdir().expect("artifacts");
    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-verify"))
        .arg("run")
        .arg("--scenario")
        .arg(fixture_path(
            "tests/fixtures/verification/scenarios/minimal-accepted.yaml",
        ))
        .arg("--workspace-root")
        .arg(workspace.path())
        .arg("--artifacts-dir")
        .arg(artifacts.path())
        .env("CORE_OPS_VERIFY_CORE_OPS_BIN", env!("CARGO_BIN_EXE_core-ops"))
        .output()
        .expect("run vm-backed verification smoke");

    assert!(
        output.status.success(),
        "vm-backed verification failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification run verify-idempotent-frontend [local]"));
}
