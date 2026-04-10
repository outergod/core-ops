use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn contributor_docs_describe_fragment_path_and_release_preparation() {
    let agents = fs::read_to_string(repo_root().join("AGENTS.md")).expect("read AGENTS");
    let docs =
        fs::read_to_string(repo_root().join("docs/development.md")).expect("read development docs");

    for contents in [agents, docs] {
        assert!(contents.contains("changes/<change-id>.md"));
        assert!(contents.contains("release_preparation: true"));
    }
}
