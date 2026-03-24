use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mount_management")
}

fn read_scenario(name: &str) -> String {
    fs::read_to_string(fixture_dir().join(name).join("scenario.yaml"))
        .expect("read mount reconcile scenario")
}

#[test]
fn reconcile_fixture_covers_invalid_and_busy_removal_paths() {
    let invalid = read_scenario("invalid-definition");
    let busy = read_scenario("busy-removal");

    assert!(invalid.contains("duplicate-target-path"));
    assert!(invalid.contains("invalid-ownership-boundary"));
    assert!(busy.contains("managed-removal"));
    assert!(busy.contains("busy-unmount-failure"));
    assert!(busy.contains("dependent-service-stop-first"));
}
