use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mount_management")
}

fn read_scenario(name: &str) -> String {
    fs::read_to_string(fixture_dir().join(name).join("scenario.yaml"))
        .expect("read mount management scenario")
}

#[test]
fn mount_management_fixture_scenarios_exist() {
    let dir = fixture_dir();
    assert!(dir.join("README.md").exists());
    assert!(dir.join("normal-nfs/scenario.yaml").exists());
    assert!(dir.join("network-automount/scenario.yaml").exists());
    assert!(dir.join("invalid-definition/scenario.yaml").exists());
    assert!(dir.join("busy-removal/scenario.yaml").exists());
}

#[test]
fn contract_fixture_covers_normal_and_automount_dependency_semantics() {
    let normal = read_scenario("normal-nfs");
    let automount = read_scenario("network-automount");

    assert!(normal.contains("named-mount-declaration"));
    assert!(normal.contains("requires-mounts-for"));
    assert!(automount.contains("automount-enabled"));
    assert!(automount.contains("explicit-unit-dependencies"));
    assert!(automount.contains("path-based-dependencies"));
}
