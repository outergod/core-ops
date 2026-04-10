use crate::integration::release_governance_support::{add_fragment, init_repo, run_release_changelog_write};

#[test]
fn changelog_command_generates_unreleased_section_from_fragments() {
    let repo = init_repo();
    add_fragment(
        repo.path(),
        "governance-feature",
        "minor",
        "Add release governance helper binary",
        false,
    );

    let output = run_release_changelog_write(repo.path());
    assert!(output.status.success());

    let changelog = std::fs::read_to_string(repo.path().join("CHANGELOG.md")).expect("read changelog");
    assert!(changelog.contains("<!-- core-ops-release:start -->"));
    assert!(changelog.contains("- Add release governance helper binary"));
}
