use std::fs;
use std::path::Path;

use core_ops::cli::status::{format_status_text, render_status_from_path};
use core_ops::io::state::persist_success_state;

fn fixture(name: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/fixtures/provenance_state").join(name);
    fs::read_to_string(path).expect("read fixture")
}

#[test]
fn status_output_reflects_canonical_success_snapshot_contents() {
    let output = format_status_text(&fixture("valid-success.json"));

    assert!(output.starts_with("provenance\n"));
    assert!(output.contains("\"repository\": \"file:///var/lib/core-ops/repo\""));
    assert!(output.contains("\"requested_ref\": \"main\""));
    assert!(output.contains("\"status\": \"success\""));
}

#[test]
fn status_output_reflects_never_run_snapshot_contents() {
    let output = format_status_text(&fixture("valid-never-run.json"));

    assert!(output.contains("\"status\": \"never_run\""));
    assert!(output.contains("\"generation\": 0"));
}

#[test]
fn status_output_reflects_in_progress_snapshot_contents() {
    let output = format_status_text(
        r#"{
  "schema_version": 1,
  "controller": {
    "version": "0.1.0",
    "revision": "8f3c2ab",
    "build_time": "2026-03-23T10:00:00Z",
    "tree_state": "clean"
  },
  "desired_state": {
    "repository": "file:///var/lib/core-ops/repo",
    "requested_ref": "main",
    "last_observed_revision": "c98dd10",
    "last_observed_at": "2026-03-23T10:07:00Z"
  },
  "reconciliation": {
    "generation": 12,
    "status": "in_progress",
    "running": true,
    "last_attempted_revision": "c98dd10",
    "last_applied_revision": "a42be91",
    "last_started_at": "2026-03-23T10:07:01Z",
    "last_finished_at": null,
    "attempted_observed_divergence": null
  }
}"#,
    );

    assert!(output.contains("\"status\": \"in_progress\""));
    assert!(output.contains("\"running\": true"));
    assert!(output.contains("\"last_applied_revision\": \"a42be91\""));
}

#[test]
fn status_output_is_stable_for_unchanged_snapshot_contents() {
    let contents = fixture("valid-success.json");

    let first = format_status_text(&contents);
    let second = format_status_text(&contents);

    assert_eq!(first, second);
}

#[test]
fn status_output_reports_absent_for_invalid_or_missing_snapshot() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let invalid = root.join("tests/fixtures/provenance_state/invalid-partial.json");
    let missing = root.join("tests/fixtures/provenance_state/missing.json");

    let invalid_output = render_status_from_path(&invalid);
    let missing_output = render_status_from_path(&missing);

    assert!(invalid_output.contains("\"status\": \"absent\""));
    assert!(missing_output.contains("\"status\": \"absent\""));
}

#[test]
fn status_output_rebuilds_after_invalid_snapshot_is_replaced() {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "core_ops_status_rebuild_{}.json",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix epoch")
            .as_nanos()
    ));
    fs::write(&path, "{\n  \"schema_version\": 1,\n  \"controller\": {").expect("write invalid");

    let absent = render_status_from_path(&path);
    assert!(absent.contains("\"status\": \"absent\""));

    persist_success_state(&path, "file:///var/lib/core-ops/repo", "main", "deadbeef")
        .expect("rebuild snapshot");
    let rebuilt = render_status_from_path(&path);

    assert!(rebuilt.contains("\"status\": \"success\""));
    assert!(rebuilt.contains("\"last_applied_revision\": \"deadbeef\""));

    let _ = fs::remove_file(path);
}
