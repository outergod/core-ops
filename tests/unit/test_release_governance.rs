use core_ops::core::release_governance::{
    classify_version_bump, parse_cargo_version, promote_changelog, render_generated_changelog,
    ReleaseFragment, ReleaseFragmentFrontMatter, ReleaseIntent,
};

#[test]
fn classifies_patch_minor_and_major_version_bumps() {
    assert_eq!(
        classify_version_bump("0.6.0", "0.6.1").expect("patch"),
        Some(ReleaseIntent::Patch)
    );
    assert_eq!(
        classify_version_bump("0.6.0", "0.7.0").expect("minor"),
        Some(ReleaseIntent::Minor)
    );
    assert_eq!(
        classify_version_bump("0.6.0", "1.0.0").expect("major"),
        Some(ReleaseIntent::Major)
    );
}

#[test]
fn parses_cargo_version_from_manifest_contents() {
    let contents = "[package]\nname = \"fixture\"\nversion = \"0.6.0\"\n";
    assert_eq!(
        parse_cargo_version(contents).expect("parse version"),
        "0.6.0".to_string()
    );
}

#[test]
fn parses_cargo_version_from_package_table_not_dependencies() {
    let contents = "[dependencies]\nsome-crate = { version = \"1.2.3\" }\n\n[package]\nname = \"fixture\"\nversion = \"0.6.0\"\n";
    assert_eq!(
        parse_cargo_version(contents).expect("parse version"),
        "0.6.0".to_string()
    );
}

#[test]
fn parse_cargo_version_handles_no_spaces_around_equals() {
    let contents = "[package]\nname=\"fixture\"\nversion=\"0.6.0\"\n";
    assert_eq!(
        parse_cargo_version(contents).expect("parse version"),
        "0.6.0".to_string()
    );
}

#[test]
fn parse_cargo_version_handles_extra_spaces_around_equals() {
    let contents = "[package]\nname = \"fixture\"\nversion  =  \"0.6.0\"\n";
    assert_eq!(
        parse_cargo_version(contents).expect("parse version"),
        "0.6.0".to_string()
    );
}

#[test]
fn parse_cargo_version_handles_tab_indented_keys() {
    let contents = "[package]\n\tname = \"fixture\"\n\tversion = \"0.6.0\"\n";
    assert_eq!(
        parse_cargo_version(contents).expect("parse version"),
        "0.6.0".to_string()
    );
}

#[test]
fn parse_cargo_version_fails_when_package_table_is_absent() {
    let contents = "[dependencies]\nsome-crate = { version = \"1.2.3\" }\n";
    assert!(parse_cargo_version(contents).is_err());
}

#[test]
fn renders_managed_unreleased_section_from_fragments() {
    let existing = "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n## [Unreleased]\n\n<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->\n\n## [0.6.0] - 2026-04-07\n";
    let fragment = ReleaseFragment {
        path: "changes/example.md".to_string(),
        front_matter: ReleaseFragmentFrontMatter {
            change_id: "example".to_string(),
            release_intent: ReleaseIntent::Minor,
            summary: "Add helper binary".to_string(),
            scope: Some("release-governance".to_string()),
            release_preparation: false,
        },
        body: String::new(),
    };

    let rendered =
        render_generated_changelog(existing, &[fragment]).expect("render changelog");
    assert!(rendered.contains("### Changed"));
    assert!(rendered.contains("- Add helper binary"));
    assert!(rendered.contains("## [0.6.0] - 2026-04-07"));
}

#[test]
fn promote_changelog_moves_unreleased_body_to_versioned_section() {
    let existing = "# Changelog\n\
                    \n\
                    ## [Unreleased]\n\
                    \n\
                    <!-- core-ops-release:start -->\n\
                    ### Changed\n\
                    \n\
                    - Some bullet\n\
                    <!-- core-ops-release:end -->\n\
                    \n\
                    ## [0.1.0] - 2026-01-01\n\
                    \n\
                    ### Added\n\
                    \n\
                    - Initial\n";
    let promoted = promote_changelog(existing, "0.2.0", "2026-05-03").expect("promote");
    // Unreleased markers preserved but emptied.
    assert!(promoted.contains(
        "## [Unreleased]\n\n<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->\n"
    ));
    // Versioned section inserted with the moved body.
    assert!(promoted.contains("## [0.2.0] - 2026-05-03\n\n### Changed\n\n- Some bullet\n"));
    // Tail (existing [0.1.0]) preserved with one blank line separator.
    assert!(promoted.contains("- Some bullet\n\n## [0.1.0] - 2026-01-01\n"));
}

#[test]
fn promote_changelog_is_idempotent_when_version_already_exists() {
    let already_promoted =
        "## [Unreleased]\n\n<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->\n\n\
         ## [2.0.0] - 2026-05-03\n\n### Changed\n\n- Bullet\n";
    let result = promote_changelog(already_promoted, "2.0.0", "2026-05-04").expect("idempotent");
    assert_eq!(result, already_promoted);
}

#[test]
fn promote_changelog_handles_empty_unreleased_block() {
    // A release-prep PR populates [Unreleased] only via a fragment
    // with `release_preparation: true` (excluded from rendering), so
    // the markers can be empty by the time promote runs. The new
    // versioned section must still land cleanly.
    let existing = "# Changelog\n\
                    \n\
                    ## [Unreleased]\n\
                    \n\
                    <!-- core-ops-release:start -->\n\
                    <!-- core-ops-release:end -->\n\
                    \n\
                    ## [0.1.0] - 2026-01-01\n\
                    \n\
                    ### Added\n\
                    \n\
                    - Initial\n";
    let promoted = promote_changelog(existing, "0.2.0", "2026-05-03").expect("promote");
    assert!(promoted.contains("## [0.2.0] - 2026-05-03\n\n## [0.1.0]"));
    // Markers still present, still empty.
    assert!(promoted.contains(
        "## [Unreleased]\n\n<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->\n"
    ));
}

#[test]
fn promote_changelog_rejects_changelog_missing_unreleased_section() {
    let bad = "# Changelog\n\n## [0.1.0] - 2026-01-01\n";
    let err = promote_changelog(bad, "0.2.0", "2026-05-03")
        .expect_err("missing [Unreleased] must error");
    assert!(err.message.contains("[Unreleased]"));
}

#[test]
fn promote_changelog_rejects_when_markers_missing() {
    let bad = "## [Unreleased]\n\n### Changed\n\n- Bullet\n\n## [0.1.0] - 2026-01-01\n";
    let err = promote_changelog(bad, "0.2.0", "2026-05-03")
        .expect_err("missing markers must error");
    assert!(err.message.contains("marker"));
}
