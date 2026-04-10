use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn ci_workflow_exposes_stable_release_governance_check() {
    let contents = fs::read_to_string(repo_root().join(".github/workflows/ci.yml"))
        .expect("read ci workflow");

    for snippet in [
        "core-ops-release",
        "Release Governance",
        "cargo run --bin core-ops-release -- validate",
    ] {
        assert!(contents.contains(snippet), "missing workflow snippet: {snippet}");
    }
}
