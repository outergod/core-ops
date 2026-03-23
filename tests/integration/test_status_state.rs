use std::fs;
use std::path::Path;

use core_ops::cli::status::format_status_text;

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
