use crate::integration::release_governance_support::{
    add_fragment, head, init_repo, run_release_validate, write_file,
};

fn write_fragment_with_blank_summary(root: &std::path::Path, change_id: &str) {
    write_file(
        root,
        &format!("changes/{change_id}.md"),
        &format!(
            "---\nchange_id: {change_id}\nrelease_intent: patch\nsummary: \"\"\nscope: governance\nrelease_preparation: false\n---\n"
        ),
    );
}

#[test]
fn unclassified_path_is_treated_as_releasable() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "flake.nix", "{ outputs = {}; }\n");

    let output = run_release_validate(repo.path(), &base, true);
    assert!(!output.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "releasable");
}

#[test]
fn fragment_with_blank_summary_is_rejected() {
    let repo = init_repo();
    let base = head(repo.path());
    write_file(repo.path(), "src/lib.rs", "pub fn baseline() -> &'static str { \"v2\" }\n");
    write_fragment_with_blank_summary(repo.path(), "blank-summary");
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );

    let output = run_release_validate(repo.path(), &base, false);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stderr.contains("blank summary") || stdout.contains("blank summary"),
        "expected blank summary error, got stderr={stderr} stdout={stdout}"
    );
}

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
