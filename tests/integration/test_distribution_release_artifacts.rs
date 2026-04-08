use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn release_binary_workflow_includes_license_and_metadata_outputs() {
    let contents = fs::read_to_string(repo_root().join(".github/workflows/release-binary.yml"))
        .expect("read release-binary workflow");
    let cargo_config = fs::read_to_string(repo_root().join(".cargo/config.toml"))
        .expect("read cargo config");

    for snippet in [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "rustup target add",
        "gcc-aarch64-linux-gnu",
        "binutils-aarch64-linux-gnu",
        "libc6-dev-arm64-cross",
        "artifact_arch=\"amd64\"",
        "artifact_arch=\"arm64\"",
        "cp specs/002-systemd-agent/contracts/systemd/core-ops.service dist/core-ops.service",
        "cp specs/002-systemd-agent/contracts/systemd/core-ops.timer dist/core-ops.timer",
        "cp LICENSE dist/LICENSE",
        "cp CHANGELOG.md dist/CHANGELOG.md",
        "cp README.md dist/README.md",
        "core-ops.service core-ops.timer LICENSE CHANGELOG.md README.md",
        "release-metadata.json",
        "dist/SHA256SUMS-${artifact_arch}",
    ] {
        assert!(contents.contains(snippet), "missing workflow snippet: {snippet}");
    }

    assert!(cargo_config.contains("[target.aarch64-unknown-linux-gnu]"));
    assert!(cargo_config.contains("linker = \"aarch64-linux-gnu-gcc\""));
}

#[test]
fn release_artifact_metadata_mentions_governing_license_and_architectures() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README");
    let license = fs::read_to_string(repo_root().join("LICENSE")).expect("read LICENSE");
    let metadata = fs::read_to_string(repo_root().join("tests/fixtures/distribution/release-metadata.json"))
        .expect("read release metadata");

    assert!(readme.contains("AGPLv3+"));
    assert!(readme.contains("x86_64"));
    assert!(readme.contains("aarch64"));
    assert!(license.contains("GNU Affero General Public License"));
    assert!(metadata.contains("x86_64 raw binary"));
    assert!(metadata.contains("aarch64 raw binary"));
}
