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
