use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn readme_contains_required_entrypoint_sections() {
    let contents = fs::read_to_string(repo_root().join("README.md")).expect("read README");

    // Spec/018 FR-001 12-section ordering (post-restructure).
    for heading in [
        "# CoreOps",
        "## 30-second mental model",
        "## Architecture",
        "## What using CoreOps feels like",
        "## Real-world examples",
        "## Quick start",
        "## Why CoreOps exists",
        "## What CoreOps is not",
        "## Trust and release model",
        "## AI authorship",
        "## Target audience · License · Further reading",
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
