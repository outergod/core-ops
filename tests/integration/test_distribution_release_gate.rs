use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn distribution_gate_is_split_between_public_ci_and_protected_e2e() {
    let ci_contents =
        fs::read_to_string(repo_root().join(".github/workflows/ci.yml")).expect("read ci workflow");
    let e2e_contents = fs::read_to_string(repo_root().join(".github/workflows/e2e-gate.yml"))
        .expect("read e2e gate workflow");

    for snippet in [
        "cargo build --locked",
        "cargo test",
        "cargo clippy --all-targets -- -D warnings",
    ] {
        assert!(
            ci_contents.contains(snippet),
            "missing CI workflow snippet: {snippet}"
        );
    }

    for snippet in [
        "cargo build --locked --bin core-ops --bin core-ops-verify",
        "CORE_OPS_VERIFY_CORE_OPS_BIN=$GITHUB_WORKSPACE/target/debug/core-ops",
        "target/debug/core-ops-verify run",
        "--accepted-dir tests/fixtures/verification/scenarios",
        "target/debug/core-ops-verify validate",
        "test_evaluation_determinism",
        "release-gate-environment.json",
        "environment: homelab-e2e",
        "workflow_dispatch",
    ] {
        assert!(
            e2e_contents.contains(snippet),
            "missing E2E workflow snippet: {snippet}"
        );
    }
}

#[test]
fn e2e_gate_workflow_validates_environment_identity_against_declared_values() {
    let contents = fs::read_to_string(repo_root().join(".github/workflows/e2e-gate.yml"))
        .expect("read e2e gate workflow");

    for snippet in [
        "target/debug/core-ops-verify validate-environment",
        "--fixture tests/fixtures/distribution/release-gate-environment.json",
        "--expected-name \"$CORE_OPS_VERIFY_ENVIRONMENT_NAME\"",
        "--expected-version \"$CORE_OPS_VERIFY_ENVIRONMENT_VERSION\"",
        "--actual-name \"$CORE_OPS_ACTUAL_VERIFY_ENVIRONMENT_NAME\"",
        "--actual-version \"$CORE_OPS_ACTUAL_VERIFY_ENVIRONMENT_VERSION\"",
        "--actual-runner-ref \"$CORE_OPS_ACTUAL_VERIFY_RUNNER_REF\"",
        "--actual-system-class \"$CORE_OPS_ACTUAL_VERIFY_SYSTEM_CLASS\"",
        "must be set by protected runner configuration",
    ] {
        assert!(contents.contains(snippet), "missing workflow snippet: {snippet}");
    }

    for forbidden in [
        "CORE_OPS_ACTUAL_VERIFY_ENVIRONMENT_NAME: ",
        "CORE_OPS_ACTUAL_VERIFY_ENVIRONMENT_VERSION: ",
        "CORE_OPS_ACTUAL_VERIFY_RUNNER_REF: ",
        "CORE_OPS_ACTUAL_VERIFY_SYSTEM_CLASS: ",
    ] {
        assert!(
            !contents.contains(forbidden),
            "workflow should not hardcode runtime identity: {forbidden}"
        );
    }
}

#[test]
fn e2e_gate_builds_and_pins_core_ops_binary_before_vm_runs() {
    let contents = fs::read_to_string(repo_root().join(".github/workflows/e2e-gate.yml"))
        .expect("read e2e gate workflow");

    for snippet in [
        "cargo build --locked --bin core-ops --bin core-ops-verify",
        "echo \"CORE_OPS_VERIFY_CORE_OPS_BIN=$GITHUB_WORKSPACE/target/debug/core-ops\" >> \"$GITHUB_ENV\"",
        "target/debug/core-ops-verify run",
    ] {
        assert!(contents.contains(snippet), "missing workflow snippet: {snippet}");
    }

    assert!(
        !contents.contains("cargo run --bin core-ops-verify -- run"),
        "workflow should not resolve the guest binary implicitly from cargo run"
    );
}

#[test]
fn release_gate_environment_fixture_is_versioned_and_drift_detectable() {
    let fixture = fs::read_to_string(
        repo_root().join("tests/fixtures/distribution/release-gate-environment.json"),
    )
    .expect("read environment fixture");
    let parsed: serde_json::Value = serde_json::from_str(&fixture).expect("parse fixture");

    assert_eq!(parsed["system_class"], "Fedora CoreOS");
    assert!(parsed["runner_definition_ref"].as_str().is_some());
    assert!(parsed["version_marker"].as_str().is_some());
    assert!(parsed["drift_detection_basis"].as_str().is_some());
}
