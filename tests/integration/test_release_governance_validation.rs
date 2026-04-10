use std::process::Command;

use crate::integration::release_governance_support::{
    add_fragment, head, init_repo, run_release_validate, run_release_validate_with_head_ref,
    run_git, write_file,
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
fn head_ref_without_base_ref_is_rejected() {
    let repo = init_repo();

    let output = Command::new(env!("CARGO_BIN_EXE_core-ops-release"))
        .current_dir(repo.path())
        .arg("validate")
        .arg("--repo-root")
        .arg(repo.path())
        .arg("--head-ref")
        .arg("HEAD")
        .output()
        .expect("run validate");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--head-ref requires --base-ref"),
        "expected rejection message, got: {stderr}"
    );
}

#[test]
fn untracked_working_tree_files_are_excluded_from_ref_to_ref_validation() {
    let repo = init_repo();
    let base = head(repo.path());

    // Commit only a docs change — this is the PR being validated (exempt).
    write_file(repo.path(), "docs/note.md", "# note\n");
    run_git(repo.path(), &["add", "docs/note.md"]);
    run_git(repo.path(), &["commit", "-m", "add docs note"]);
    let head_ref = head(repo.path());

    // Add a releasable untracked file to the working tree (not committed).
    // With the bug, this file would be included in the diff and force a
    // releasable classification; with the fix it must be ignored.
    write_file(repo.path(), "flake.nix", "{ outputs = {}; }\n");

    let output = run_release_validate_with_head_ref(repo.path(), &base, &head_ref, true);
    assert!(
        output.status.success(),
        "untracked files must not affect ref-to-ref validation; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(parsed["effective_classification"], "exempt");
}

#[test]
fn head_ref_governance_files_take_precedence_over_working_tree() {
    let repo = init_repo();
    let base = head(repo.path());

    // Commit a src change without any release metadata — this becomes head_ref.
    write_file(
        repo.path(),
        "src/lib.rs",
        "pub fn baseline() -> &'static str { \"changed\" }\n",
    );
    crate::integration::release_governance_support::run_git(repo.path(), &["add", "src/lib.rs"]);
    crate::integration::release_governance_support::run_git(
        repo.path(),
        &["commit", "-m", "change src without metadata"],
    );
    let head_without_metadata = head(repo.path());

    // Put valid metadata in the working tree only (not committed).
    add_fragment(repo.path(), "working-tree-fragment", "patch", "Working tree only", false);
    write_file(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.6.1\"\nedition = \"2021\"\n",
    );

    // Validate against the committed head (which lacks metadata).
    // If governance reads from the working tree the test would incorrectly pass;
    // with the fix it must fail because the committed head has no metadata.
    let output =
        run_release_validate_with_head_ref(repo.path(), &base, &head_without_metadata, false);
    assert!(
        !output.status.success(),
        "governance must fail: head_ref commit lacks release metadata"
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
