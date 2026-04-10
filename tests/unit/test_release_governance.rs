use core_ops::core::release_governance::{
    classify_version_bump, parse_cargo_version, render_generated_changelog, ReleaseFragment,
    ReleaseFragmentFrontMatter, ReleaseIntent,
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
