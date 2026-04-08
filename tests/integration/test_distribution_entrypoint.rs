use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn readme_contains_required_entrypoint_sections() {
    let contents = fs::read_to_string(repo_root().join("README.md")).expect("read README");

    for heading in [
        "# CoreOps",
        "## What is CoreOps?",
        "## Why CoreOps Exists",
        "## What CoreOps Is Not",
        "## Credibility",
        "## Target Audience",
        "## Supported Systems",
        "## AI Authorship",
        "## Minimal Trust Story",
        "## Installation (Current Phase)",
        "## Release & Verification Model",
    ] {
        assert!(contents.contains(heading), "missing heading: {heading}");
    }
}

#[test]
fn readme_states_supported_and_unsupported_system_classes() {
    let contents = fs::read_to_string(repo_root().join("README.md")).expect("read README");

    assert!(contents.contains("**Supported:** Fedora CoreOS"));
    assert!(contents.contains("other systemd-based hosts (untested)"));
    assert!(contents.contains("**Unsupported:** non-systemd environments"));
    assert!(contents.contains("running CoreOps from"));
    assert!(contents.contains("a container is not a supported consumption method"));
}
