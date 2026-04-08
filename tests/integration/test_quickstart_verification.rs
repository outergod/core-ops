use std::fs;
use std::path::Path;

#[test]
fn quickstart_mentions_local_debug_and_ci_verification_flows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/008-e2e-verification-harness/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("## Local Execution"));
    assert!(contents.contains("## CI Gating"));
    assert!(contents.contains("--scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml"));
    assert!(contents.contains("--accepted-dir tests/fixtures/verification/scenarios"));
    assert!(contents.contains("--ci --json"));
    assert!(contents.contains("debug mode"));
    assert!(contents.contains("--pause-before-teardown"));
    assert!(contents.contains("retained artifact bundle"));
}

#[test]
fn quickstart_mentions_vm_backed_runtime_selection_and_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/008-e2e-verification-harness/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("VM-backed disposable-machine execution is the authoritative verification path"));
    assert!(contents.contains("qemu:///system"));
    assert!(contents.contains("CORE_OPS_VERIFY_VM_HOST"));
    assert!(contents.contains("CORE_OPS_VERIFY_LIBVIRT_URI"));
    assert!(contents.contains("CORE_OPS_VERIFY_CORE_OPS_BIN"));
    assert!(contents.contains("revision-selection basis"));
    assert!(contents.contains("per-scenario revision-under-test provenance"));
    assert!(contents.contains("synthetic execution as a public replacement for VM-backed verification"));
}

#[test]
fn quickstart_mentions_repository_evolution_and_regression_authoring() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/008-e2e-verification-harness/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("repository-evolution verification"));
    assert!(contents.contains("Git-history sequence"));
    assert!(contents.contains("bug reproductions"));
    assert!(contents.contains("permanent regression scenarios"));
    assert!(contents.contains("named environment and policy profiles"));
    assert!(contents.contains("semantic step actions"));
    assert!(contents.contains("Verification Guidance"));
}

#[test]
fn distribution_quickstart_mentions_fresh_install_and_operator_verification_flow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/010-distribution-readiness/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read distribution quickstart");

    assert!(contents.contains("Fresh Fedora CoreOS system"));
    assert!(contents.contains("No undeclared host preparation"));
    assert!(contents.contains("Download the current published CoreOps binary artifact"));
    assert!(contents.contains("Run the documented first command"));
    assert!(contents.contains("Execute the documented minimal operator verification flow"));
    assert!(contents.contains("Version identity is visible and consistent across declared surfaces"));
}
