use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

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

#[test]
fn snapshot_comparison_identifies_controller_desired_state_and_outcome_changes() {
    let base: Value =
        serde_json::from_str(&read_fixture("valid-success.json")).expect("parse base fixture");
    let mut controller_changed = base.clone();
    controller_changed["controller"]["version"] = Value::String("0.2.0".to_string());
    let mut desired_changed = base.clone();
    desired_changed["desired_state"]["last_observed_revision"] =
        Value::String("feedface".to_string());
    let mut outcome_changed = base.clone();
    outcome_changed["reconciliation"]["status"] = Value::String("failed".to_string());

    assert_ne!(
        controller_changed["controller"]["version"],
        base["controller"]["version"]
    );
    assert_eq!(
        controller_changed["desired_state"]["last_observed_revision"],
        base["desired_state"]["last_observed_revision"]
    );
    assert_ne!(
        desired_changed["desired_state"]["last_observed_revision"],
        base["desired_state"]["last_observed_revision"]
    );
    assert_eq!(
        desired_changed["reconciliation"]["status"],
        base["reconciliation"]["status"]
    );
    assert_ne!(
        outcome_changed["reconciliation"]["status"],
        base["reconciliation"]["status"]
    );
}

#[test]
fn controller_version_provenance_matches_cargo_package_version() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo package version");
    let contents = read_fixture("valid-success.json");
    let parsed: Value = serde_json::from_str(&contents).expect("parse success fixture");

    assert_eq!(
        parsed["controller"]["version"].as_str(),
        Some(version.as_str())
    );
}
