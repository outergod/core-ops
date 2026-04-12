use core_ops::build_info::ReleaseMetadata;
use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn credibility_surface_matches_release_metadata_fixture() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    let metadata: ReleaseMetadata = serde_json::from_str(
        &fs::read_to_string(root.join("tests/fixtures/distribution/release-metadata.json"))
            .expect("read release metadata"),
    )
    .expect("parse release metadata");

    assert!(readme.contains("## Credibility"));
    // Live badges replaced the static release identity, gate status, and verification status values
    assert!(readme.contains("actions/workflows/ci.yml/badge.svg"), "missing CI badge");
    assert!(readme.contains("actions/workflows/e2e-gate.yml/badge.svg"), "missing E2E gate badge");
    assert!(readme.contains("img.shields.io/github/v/release/"), "missing release version badge");
    // Static artifact availability rows remain alongside the live badges
    for artifact in &metadata.artifact_availability {
        assert!(readme.contains(artifact), "missing artifact {artifact}");
    }
}

#[test]
fn credibility_snapshot_fixture_lists_stable_sections() {
    let contents = fs::read_to_string(
        repo_root().join("tests/fixtures/distribution/entrypoint-snapshot.md"),
    )
    .expect("read entrypoint snapshot");

    assert!(contents.contains("Credibility"));
    assert!(contents.contains("Installation (Current Phase)"));
    assert!(contents.contains("Release & Verification Model"));
    assert!(contents.contains("Supported Systems"));
}
