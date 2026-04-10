use crate::integration::release_governance_support::{
    add_fragment, head, init_repo, run_release_validate, write_file,
};

#[test]
fn validate_fails_when_additive_change_declares_patch() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "src/new_capability.rs", "pub fn added() {}\n");
    add_fragment(
        repo.path(),
        "additive-change",
        "patch",
        "Add helper capability",
        false,
    );
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );
    let changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md"))
        .expect("read changelog")
        .replace(
            "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
            "<!-- core-ops-release:start -->\n### Changed\n\n- Add helper capability\n<!-- core-ops-release:end -->",
        );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Required bump: minor"));
    assert!(stdout.contains("declared release intent patch does not match required minor"));
}

#[test]
fn validate_requires_major_for_machine_readable_contract_changes() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/distribution/release-metadata.json",
        "{\"schema\": 2}\n",
    );
    add_fragment(
        repo.path(),
        "breaking-contract",
        "minor",
        "Change release metadata contract",
        false,
    );
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.7.0\"\nedition = \"2021\"\n",
    );
    let changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md"))
        .expect("read changelog")
        .replace(
            "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
            "<!-- core-ops-release:start -->\n### Changed\n\n- Change release metadata contract\n<!-- core-ops-release:end -->",
        );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Required bump: major"));
}

#[test]
fn release_metadata_version_sync_does_not_force_major() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/distribution/release-metadata.json",
        "{\n  \"latest_release_identity\": \"0.7.0-dev\",\n  \"release_gate_status\": \"design-contract\",\n  \"accepted_verification_status\": \"design-contract\",\n  \"artifact_availability\": [\"x86_64 raw binary\"],\n  \"verification_environment\": \"fedora-coreos-self-hosted@2026-04-fcos\",\n  \"credibility_location\": \"README.md#credibility\"\n}\n",
    );
    add_fragment(
        repo.path(),
        "version-sync",
        "minor",
        "Add helper capability",
        false,
    );
    write_file(repo.path(), "src/new_capability.rs", "pub fn added() {}\n");
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.7.0\"\nedition = \"2021\"\n",
    );
    let changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md"))
        .expect("read changelog")
        .replace(
            "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
            "<!-- core-ops-release:start -->\n### Changed\n\n- Add helper capability\n<!-- core-ops-release:end -->",
        );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
}

#[test]
fn accepted_verification_corpus_changes_require_at_least_patch() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/verification/scenarios/example.yaml",
        "scenario_id: example\n",
    );
    add_fragment(
        repo.path(),
        "accepted-scenario",
        "patch",
        "Tighten accepted verification coverage",
        false,
    );
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );
    let changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md"))
        .expect("read changelog")
        .replace(
            "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
            "<!-- core-ops-release:start -->\n### Changed\n\n- Tighten accepted verification coverage\n<!-- core-ops-release:end -->",
        );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(output.status.success());
}
