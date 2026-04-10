use crate::integration::release_governance_support::{
    add_fragment, head, init_repo, run_release_validate, write_file,
};

fn full_release_metadata(repo_path: &std::path::Path, change_id: &str, intent: &str, summary: &str) {
    let before_version = "0.6.0";
    let after_version = "0.6.1";
    add_fragment(repo_path, change_id, intent, summary, false);
    write_file(
        repo_path,
        "Cargo.toml",
        &format!("[package]\nname = \"fixture\"\nversion = \"{after_version}\"\nedition = \"2021\"\n"),
    );
    let changelog = std::fs::read_to_string(repo_path.join("CHANGELOG.md"))
        .expect("read changelog")
        .replace(
            "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
            &format!(
                "<!-- core-ops-release:start -->\n### Changed\n\n- {summary}\n<!-- core-ops-release:end -->"
            ),
        );
    std::fs::write(repo_path.join("CHANGELOG.md"), changelog).expect("write changelog");
    let _ = before_version;
}

#[test]
fn exempt_only_docs_changes_pass_without_metadata() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "docs/note.md", "# docs\n");

    let output = run_release_validate(repo.path(), &base, true);
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "exempt");
}

#[test]
fn metadata_only_changes_fail_without_release_preparation() {
    let repo = init_repo();
    let base = head(repo.path());
    add_fragment(
        repo.path(),
        "metadata-only",
        "patch",
        "Prepare release metadata",
        false,
    );
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );

    let output = run_release_validate(repo.path(), &base, false);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("metadata-only changes require release_preparation: true"));
}

#[test]
fn provenance_state_json_fixture_is_releasable() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/provenance_state/valid-success.json",
        "{\"schema_version\": 1, \"last_run\": \"success\"}\n",
    );

    let output = run_release_validate(repo.path(), &base, true);
    assert!(!output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
}

#[test]
fn provenance_state_json_fixture_passes_with_full_release_metadata() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/provenance_state/valid-success.json",
        "{\"schema_version\": 1, \"last_run\": \"success\"}\n",
    );
    full_release_metadata(repo.path(), "fix-provenance-contract", "patch", "Fix provenance state contract fixture");

    let output = run_release_validate(repo.path(), &base, true);
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
}

#[test]
fn verification_artifact_json_fixture_is_releasable() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/verification/artifacts/run-result-passed.json",
        "{\"status\": \"passed\"}\n",
    );

    let output = run_release_validate(repo.path(), &base, true);
    assert!(!output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
}

#[test]
fn verification_artifact_json_fixture_passes_with_full_release_metadata() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(
        repo.path(),
        "tests/fixtures/verification/artifacts/run-result-passed.json",
        "{\"status\": \"passed\"}\n",
    );
    full_release_metadata(repo.path(), "fix-verification-artifact", "patch", "Fix verification artifact contract fixture");

    let output = run_release_validate(repo.path(), &base, true);
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
}

#[test]
fn metadata_only_changes_pass_with_release_preparation() {
    let repo = init_repo();
    let base = head(repo.path());
    add_fragment(
        repo.path(),
        "release-prep",
        "patch",
        "Prepare the next release",
        true,
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
            "<!-- core-ops-release:start -->\n### Changed\n\n- Prepare the next release\n<!-- core-ops-release:end -->",
        );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(output.status.success());
}
