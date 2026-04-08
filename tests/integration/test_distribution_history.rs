use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn changelog_uses_keep_a_changelog_structure() {
    let contents = fs::read_to_string(repo_root().join("CHANGELOG.md")).expect("read changelog");

    assert!(contents.contains("# Changelog"));
    assert!(contents.contains("The format is based on Keep a Changelog"));
    assert!(contents.contains("## [Unreleased]"));
    assert!(contents.contains("## [0.6.0]"));
}

#[test]
fn readme_links_to_changelog_for_release_history() {
    let contents = fs::read_to_string(repo_root().join("README.md")).expect("read README");
    assert!(contents.contains("[CHANGELOG.md](CHANGELOG.md)"));
}
