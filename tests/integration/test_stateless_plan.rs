//! Stateless `core-ops plan --source-repo` integration test (T021).
//!
//! Covers the FR-010..FR-016 contract surface end-to-end via the
//! cargo-built binary:
//! - (a) non-git tempdir → exit 0, `(stateless)` provenance.
//! - (b) clean git checkout → exit 0, 40-char SHA provenance.
//! - (c) dirty working tree → exit 0, `(stateless+dirty)` provenance.
//! - (d) missing `--host` → clap exit 2.
//! - (e) non-directory path → exit non-zero with helpful diagnostic.
//! - (f) `--audit-dir` honored when explicitly set (clarification Q4).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::integration::source_repo_support::git_init_commit;

fn write_minimal_layout(root: &Path) {
    let services = root.join("services/alpha/quadlet");
    std::fs::create_dir_all(&services).expect("services dir");
    std::fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("alpha.container");
    let hosts = root.join("hosts/example");
    std::fs::create_dir_all(&hosts).expect("hosts dir");
    std::fs::write(
        hosts.join("host.yaml"),
        "host: example\nservices:\n  - alpha\n",
    )
    .expect("host.yaml");
}

fn coreops() -> Command {
    Command::new(env!("CARGO_BIN_EXE_core-ops"))
}

fn run_plan(source: &Path, host: &str, quadlet_dir: &Path) -> std::process::Output {
    coreops()
        .arg("plan")
        .arg("--source-repo")
        .arg(source)
        .arg("--host")
        .arg(host)
        .arg("--quadlet-dir")
        .arg(quadlet_dir)
        .output()
        .expect("invoke core-ops")
}

fn quadlet_dir() -> PathBuf {
    tempfile::TempDir::new().expect("tempdir").keep()
}

#[test]
fn stateless_plan_against_non_git_directory_succeeds_with_stateless_sentinel() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    write_minimal_layout(tmp.path());
    let qdir = quadlet_dir();
    let output = run_plan(tmp.path(), "example", &qdir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(stateless)"),
        "expected `(stateless)` provenance in plan header, got:\n{stdout}"
    );
}

#[test]
fn stateless_plan_against_clean_git_checkout_records_full_sha() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    write_minimal_layout(tmp.path());
    let sha = git_init_commit(tmp.path());
    assert_eq!(sha.len(), 40, "git_init_commit should return a full SHA");
    let qdir = quadlet_dir();
    let output = run_plan(tmp.path(), "example", &qdir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let short_sha = &sha[..8];
    assert!(
        stdout.contains(short_sha),
        "expected SHA prefix {short_sha} in plan header, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("(stateless)"),
        "clean checkout must not surface (stateless) sentinel, got:\n{stdout}"
    );
}

#[test]
fn stateless_plan_against_dirty_working_tree_records_dirty_sentinel() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    write_minimal_layout(tmp.path());
    let _sha = git_init_commit(tmp.path());
    // Introduce uncommitted change (untracked file).
    std::fs::write(tmp.path().join("scratch.txt"), "wip\n").expect("scratch");
    let qdir = quadlet_dir();
    let output = run_plan(tmp.path(), "example", &qdir);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(stateless+dirty)"),
        "expected `(stateless+dirty)` provenance in plan header, got:\n{stdout}"
    );
}

#[test]
fn stateless_plan_without_host_errors_with_clap_diagnostic() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    write_minimal_layout(tmp.path());
    let output = coreops()
        .arg("plan")
        .arg("--source-repo")
        .arg(tmp.path())
        .output()
        .expect("invoke core-ops");
    assert!(
        !output.status.success(),
        "plan without --host must error; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--host") || stderr.contains("host"),
        "stderr must mention --host requirement: {stderr}"
    );
}

#[test]
fn stateless_plan_against_non_directory_path_errors_with_exit_code_64() {
    // Per `contracts/cli-flag.md` Error semantics: <PATH> is not a
    // directory → exit 64 (`EX_USAGE`). Asserts both the diagnostic
    // and the documented exit status so automation can rely on it.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let file_path = tmp.path().join("a-file");
    std::fs::write(&file_path, "x").expect("write");
    let qdir = quadlet_dir();
    let output = run_plan(&file_path, "example", &qdir);
    assert!(
        !output.status.success(),
        "non-directory --source-repo must error"
    );
    assert_eq!(
        output.status.code(),
        Some(64),
        "non-directory --source-repo must exit 64 (EX_USAGE) per contracts/cli-flag.md"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not a directory") || stderr.contains("does not exist"),
        "stderr must reference path-shape error: {stderr}"
    );
}

#[test]
fn stateless_plan_against_missing_path_errors_with_exit_code_64() {
    // Per `contracts/cli-flag.md` Error semantics: <PATH> does not
    // exist → exit 64 (`EX_USAGE`).
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let missing = tmp.path().join("nope");
    let qdir = quadlet_dir();
    let output = run_plan(&missing, "example", &qdir);
    assert!(!output.status.success(), "missing --source-repo must error");
    assert_eq!(
        output.status.code(),
        Some(64),
        "missing --source-repo must exit 64 (EX_USAGE) per contracts/cli-flag.md"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not exist") || stderr.contains("missing"),
        "stderr must mention missing path: {stderr}"
    );
}

#[test]
fn stateless_plan_against_invalid_layout_errors_with_exit_code_65() {
    // Per `contracts/cli-flag.md` Error semantics: <PATH> is a
    // directory but layout is invalid → exit 65 (`EX_DATAERR`).
    // Distinct from path-shape errors (64) so automation can
    // classify malformed inputs from generic runtime failures.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    // Empty directory: passes the path-existence + is-directory
    // checks, then fails at the parser layer with MissingServicesDir.
    let qdir = quadlet_dir();
    let output = run_plan(tmp.path(), "example", &qdir);
    assert!(
        !output.status.success(),
        "invalid layout must error; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.status.code(),
        Some(65),
        "invalid layout must exit 65 (EX_DATAERR) per contracts/cli-flag.md"
    );
}

#[test]
fn stateless_plan_honors_explicit_audit_dir() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    write_minimal_layout(tmp.path());
    let qdir = quadlet_dir();
    let audit_dir = tempfile::TempDir::new().expect("audit tempdir");
    let output = coreops()
        .arg("plan")
        .arg("--source-repo")
        .arg(tmp.path())
        .arg("--host")
        .arg("example")
        .arg("--quadlet-dir")
        .arg(&qdir)
        .arg("--audit-dir")
        .arg(audit_dir.path())
        .output()
        .expect("invoke core-ops");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entries: Vec<_> = std::fs::read_dir(audit_dir.path())
        .expect("read audit dir")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect entries");
    assert!(
        !entries.is_empty(),
        "stateless plan with --audit-dir must write at least one audit record"
    );
}
