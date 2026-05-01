//! Integration tests for `scripts/migrate-legacy-source-repo.sh` per
//! T302 in `specs/016-source-repository-layout/tasks.md`.
//!
//! The fixture at `tests/fixtures/legacy_source_repo/` is a minimal
//! legacy-shaped repository covering every transformation in
//! research.md D10. Each test copies the fixture to a TempDir, runs
//! the migration script in-place, and asserts a property of the
//! resulting tree:
//!
//! - the new parser loads the migrated tree without error,
//! - `core-ops plan`'s destination set matches the hand-recorded
//!   `expected-destinations.txt`,
//! - re-running the script is idempotent (no-op exit, no diff),
//! - the variant service emerges with a generated `service.yaml`
//!   declaring the correct `config-root`.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::integration::source_repo_support::{copy_dir_recursive, git_init_commit, load_with_host};

const HOST: &str = "example-host";
const SCRIPT_REL: &str = "scripts/migrate-legacy-source-repo.sh";
const FIXTURE_REL: &str = "tests/fixtures/legacy_source_repo";

fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn migrate_script() -> PathBuf {
    project_root().join(SCRIPT_REL)
}

fn legacy_fixture() -> PathBuf {
    project_root().join(FIXTURE_REL)
}

/// Stage the fixture into a fresh TempDir so the migration runs
/// against a writable copy. `expected-destinations.txt` is excluded
/// from the staged tree (it's a contract artifact, not a layout one).
fn stage_fixture() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    copy_dir_recursive(&legacy_fixture(), tmp.path()).expect("copy fixture");
    let _ = std::fs::remove_file(tmp.path().join("expected-destinations.txt"));
    let _ = std::fs::remove_file(tmp.path().join("README.md"));
    tmp
}

fn run_migration(repo: &Path) -> std::process::Output {
    Command::new(migrate_script())
        .arg(repo)
        .output()
        .expect("run migration script")
}

fn expected_destinations() -> Vec<String> {
    let body =
        std::fs::read_to_string(legacy_fixture().join("expected-destinations.txt"))
            .expect("read expected-destinations.txt");
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn destinations_from_state(state: &core_ops::core::types::DesiredState) -> Vec<String> {
    let mut paths: Vec<String> = state
        .workloads
        .iter()
        .map(|workload| match workload.quadlet_type {
            core_ops::core::types::QuadletType::ConfigFile => {
                workload.systemd_unit_name.clone()
            }
            core_ops::core::types::QuadletType::Container
            | core_ops::core::types::QuadletType::Volume
            | core_ops::core::types::QuadletType::Network
            | core_ops::core::types::QuadletType::Pod => {
                format!("/etc/containers/systemd/{}", workload.systemd_unit_name)
            }
            core_ops::core::types::QuadletType::Socket
            | core_ops::core::types::QuadletType::Mount
            | core_ops::core::types::QuadletType::Automount
            | core_ops::core::types::QuadletType::SocketDropIn
            | core_ops::core::types::QuadletType::Timer
            | core_ops::core::types::QuadletType::Target
            | core_ops::core::types::QuadletType::Path => {
                format!("/etc/systemd/system/{}", workload.systemd_unit_name)
            }
        })
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

#[test]
fn migration_script_produces_loader_compatible_tree() {
    let staged = stage_fixture();
    let output = run_migration(staged.path());
    assert!(
        output.status.success(),
        "migration failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rev = git_init_commit(staged.path());
    let state = load_with_host(staged.path(), &rev, HOST).expect("load migrated tree");
    assert!(
        !state.workloads.is_empty(),
        "migrated tree must yield at least one workload"
    );
}

#[test]
fn migration_destinations_match_expected_set() {
    let staged = stage_fixture();
    let output = run_migration(staged.path());
    assert!(output.status.success(), "migration failed");

    let rev = git_init_commit(staged.path());
    let state = load_with_host(staged.path(), &rev, HOST).expect("load migrated tree");
    let actual = destinations_from_state(&state);
    let expected = expected_destinations();
    assert_eq!(
        actual, expected,
        "migrated destination set diverged from expected-destinations.txt"
    );
}

#[test]
fn migration_script_is_idempotent() {
    let staged = stage_fixture();
    let first = run_migration(staged.path());
    assert!(first.status.success(), "first migration must succeed");

    // Snapshot the tree post first migration.
    let snapshot_first = walk_relative(staged.path());

    let second = run_migration(staged.path());
    assert!(
        second.status.success(),
        "second migration must succeed (idempotent): stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
    let snapshot_second = walk_relative(staged.path());

    assert_eq!(
        snapshot_first, snapshot_second,
        "second migration must be a no-op; tree snapshot diverged"
    );
}

#[test]
fn migration_emits_service_yaml_for_variant_config_root() {
    let staged = stage_fixture();
    let output = run_migration(staged.path());
    assert!(output.status.success(), "migration failed");

    let manifest = staged
        .path()
        .join("services/traefik-dnschallenge/service.yaml");
    assert!(
        manifest.exists(),
        "variant service must emerge with a service.yaml at {}",
        manifest.display()
    );
    let body = std::fs::read_to_string(&manifest).expect("read service.yaml");
    assert!(
        body.contains("config-root: traefik"),
        "variant service.yaml must declare the upstream config-root, got: {body}"
    );
}

/// Walk a directory tree and return a sorted `Vec<(relative-path, sha256-of-bytes)>`
/// suitable for equality comparisons. Used by the idempotence test.
fn walk_relative(root: &Path) -> Vec<(String, String)> {
    fn collect(root: &Path, current: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(current).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            // Skip the .git directory created by git_init_commit if any
            // pre-existing test left one; the staged fixture should have
            // none, but the second-migration assertion runs before the
            // first git_init_commit in this particular test.
            if path.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            if entry.file_type().expect("file type").is_dir() {
                collect(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().display().to_string();
                let bytes = std::fs::read(&path).expect("read file");
                let hash = simple_hash(&bytes);
                out.push((rel, hash));
            }
        }
    }
    let mut out = Vec::new();
    collect(root, root, &mut out);
    out.sort();
    out
}

/// 64-bit FNV-1a hash, hex-encoded. Plenty for snapshot comparison.
fn simple_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
