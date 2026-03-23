use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/provenance_state")
}

fn read_fixture(name: &str) -> String {
    let path = fixture_dir().join(name);
    fs::read_to_string(path).expect("read provenance fixture")
}

#[test]
fn provenance_state_fixtures_exist() {
    let dir = fixture_dir();
    assert!(dir.join("README.md").exists());
    assert!(dir.join("valid-success.json").exists());
    assert!(dir.join("valid-never-run.json").exists());
    assert!(dir.join("invalid-partial.json").exists());
    assert!(dir.join("invalid-unsupported-schema.json").exists());
}

#[test]
fn valid_success_fixture_contains_required_top_level_sections() {
    let contents = read_fixture("valid-success.json");
    assert!(contents.contains("\"schema_version\""));
    assert!(contents.contains("\"controller\""));
    assert!(contents.contains("\"desired_state\""));
    assert!(contents.contains("\"reconciliation\""));
}

#[test]
fn invalid_fixture_examples_cover_partial_and_unsupported_cases() {
    let partial = read_fixture("invalid-partial.json");
    let unsupported = read_fixture("invalid-unsupported-schema.json");

    assert!(!partial.trim_end().ends_with('}'));
    assert!(unsupported.contains("\"schema_version\": 99"));
}
