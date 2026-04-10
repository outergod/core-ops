use crate::integration::release_governance_support::{
    add_fragment, head, init_repo, run_release_validate, write_file,
};

#[test]
fn validate_fails_for_releasable_changes_missing_version_and_fragment() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "src/lib.rs", "pub fn baseline() -> &'static str { \"changed\" }\n");

    let output = run_release_validate(repo.path(), &base, false);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Classification: releasable"));
    assert!(stdout.contains("Missing Artifacts:"));
    assert!(stdout.contains("Cargo.toml"));
    assert!(stdout.contains("changes/<change-id>.md"));
}

#[test]
fn validate_reports_mixed_releasable_and_exempt_deltas_in_json() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "src/lib.rs", "pub fn baseline() -> &'static str { \"changed\" }\n");
    write_file(repo.path(), "docs/note.md", "# docs only\n");

    add_fragment(
        repo.path(),
        "mixed-change",
        "patch",
        "Adjust runtime behavior",
        false,
    );
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );
    let mut changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md")).expect("changelog");
    changelog = changelog.replace(
        "<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->",
        "<!-- core-ops-release:start -->\n### Changed\n\n- Adjust runtime behavior\n<!-- core-ops-release:end -->",
    );
    std::fs::write(repo.path().join("CHANGELOG.md"), changelog).expect("write changelog");

    let output = run_release_validate(repo.path(), &base, true);
    assert!(output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
    assert_eq!(parsed["effective_bump"], "patch");
}
