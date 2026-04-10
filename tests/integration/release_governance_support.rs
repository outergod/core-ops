use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write file");
}

pub fn init_repo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().expect("temp repo");
    run_git(repo.path(), &["init"]);
    run_git(repo.path(), &["config", "user.email", "tests@example.com"]);
    run_git(repo.path(), &["config", "user.name", "CoreOps Tests"]);

    write_file(
        repo.path(),
        "Cargo.toml",
        r#"[package]
name = "fixture"
version = "0.6.0"
edition = "2021"
"#,
    );
    write_file(
        repo.path(),
        "CHANGELOG.md",
        "# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n## [Unreleased]\n\n<!-- core-ops-release:start -->\n<!-- core-ops-release:end -->\n\n## [0.6.0] - 2026-04-07\n\n### Added\n\n- Baseline release\n",
    );
    write_file(repo.path(), "README.md", "# Fixture\n");
    write_file(repo.path(), "changes/README.md", "# Release Fragments\n");
    write_file(
        repo.path(),
        "tests/fixtures/distribution/release-metadata.json",
        "{\n  \"latest_release_identity\": \"0.6.0-dev\",\n  \"release_gate_status\": \"design-contract\",\n  \"accepted_verification_status\": \"design-contract\",\n  \"artifact_availability\": [\"x86_64 raw binary\"],\n  \"verification_environment\": \"fedora-coreos-self-hosted@2026-04-fcos\",\n  \"credibility_location\": \"README.md#credibility\"\n}\n",
    );
    write_file(
        repo.path(),
        ".github/workflows/ci.yml",
        "name: PR CI\non:\n  pull_request:\n  push:\n    branches: [\"master\"]\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: cargo build --locked --bin core-ops --bin core-ops-verify --bin core-ops-release\n      - run: cargo test\n      - run: cargo clippy --all-targets -- -D warnings\n      - name: Release Governance\n        run: cargo run --bin core-ops-release -- validate --base-ref HEAD^\n",
    );
    write_file(repo.path(), "src/lib.rs", "pub fn baseline() -> &'static str { \"ok\" }\n");
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "baseline"]);
    repo
}

pub fn run_git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn head(root: &Path) -> String {
    run_git(root, &["rev-parse", "HEAD"]).trim().to_string()
}

pub fn add_fragment(
    root: &Path,
    change_id: &str,
    release_intent: &str,
    summary: &str,
    release_preparation: bool,
) -> PathBuf {
    let path = format!("changes/{change_id}.md");
    write_file(
        root,
        &path,
        &format!(
            "---\nchange_id: {change_id}\nrelease_intent: {release_intent}\nsummary: {summary}\nscope: release-governance\nrelease_preparation: {release_preparation}\n---\n"
        ),
    );
    root.join(path)
}

pub fn run_release_validate(root: &Path, base_ref: &str, json: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_core-ops-release"));
    command
        .current_dir(root)
        .arg("validate")
        .arg("--repo-root")
        .arg(root)
        .arg("--base-ref")
        .arg(base_ref);
    if json {
        command.arg("--json");
    }
    command.output().expect("run release validate")
}

pub fn run_release_changelog_write(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_core-ops-release"))
        .current_dir(root)
        .arg("changelog")
        .arg("--repo-root")
        .arg(root)
        .arg("--write")
        .output()
        .expect("run release changelog")
}
