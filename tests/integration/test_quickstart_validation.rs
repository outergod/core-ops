use std::fs;
use std::path::Path;

#[test]
fn quickstart_mentions_systemd_units_and_env() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/002-systemd-agent/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("core-ops.service"));
    assert!(contents.contains("core-ops.timer"));
    assert!(contents.contains("CORE_OPS_REPO"));
    assert!(contents.contains("CORE_OPS_REV"));
}

#[test]
fn quickstart_mentions_layered_overrides_flow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/003-layered-overrides/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("services/"));
    assert!(contents.contains("hosts/"));
    assert!(contents.contains("CORE_OPS_HOST"));
    assert!(contents.contains("Evaluation Flow"));
}

#[test]
fn quickstart_mentions_provenance_status_and_version_review_flow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/004-reconcile-provenance/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("core-ops status"));
    assert!(contents.contains("/var/lib/core-ops/status.json"));
    assert!(contents.contains("--force-no-state"));
    assert!(contents.contains("Cargo.toml"));
    assert!(contents.contains("minor version review"));
    assert!(contents.contains("0.2.0 -> 0.3.0"));
}

#[test]
fn quickstart_mentions_mount_management_workflow_and_version_outcome() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let quickstart = root.join("specs/005-native-mount-management/quickstart.md");
    let contents = fs::read_to_string(&quickstart).expect("read quickstart");

    assert!(contents.contains("named mount declaration"));
    assert!(contents.contains("RequiresMountsFor"));
    assert!(contents.contains(".automount"));
    assert!(contents.contains("busy"));
    assert!(contents.contains("0.3.0 -> 0.4.0"));
}
