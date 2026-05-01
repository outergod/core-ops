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

/// Codex P1 on PR #28 (53404ab follow-up): drop-ins must follow their
/// base unit's payload kind through migration. A legacy
/// `services/<svc>/quadlet-overrides/<unit>.socket.d/` (where socket
/// is now a systemd-kind extension) must land at
/// `services/<svc>/systemd/<unit>.socket.d/`, not under `quadlet/`.
/// Same rule for host overrides under `hosts/<h>/overrides/quadlet/`.
/// Without this routing, the migrated tree fails the new parser's
/// cross-kind drop-in check.
#[test]
fn migration_routes_systemd_kind_dropins_to_systemd_subtree() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    // Service with a base socket (Phase 1.a will move it to systemd/),
    // a legacy quadlet-overrides drop-in TARGETING THE SOCKET, and a
    // base container.
    std::fs::create_dir_all(repo.join("services/web/quadlet")).unwrap();
    std::fs::create_dir_all(
        repo.join("services/web/quadlet-overrides/web.socket.d"),
    )
    .unwrap();
    std::fs::create_dir_all(
        repo.join("services/web/quadlet-overrides/web.container.d"),
    )
    .unwrap();
    std::fs::write(
        repo.join("services/web/quadlet/web.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/web/quadlet/web.socket"),
        "[Socket]\nListenStream=80\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/web/quadlet-overrides/web.socket.d/10-defaults.conf"),
        "[Socket]\nNoDelay=true\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/web/quadlet-overrides/web.container.d/10-resources.conf"),
        "[Service]\nMemoryMax=256M\n",
    )
    .unwrap();
    // Host with the same kind of cross-kind override.
    std::fs::create_dir_all(
        repo.join("hosts/host-a/overrides/quadlet/web.socket.d"),
    )
    .unwrap();
    std::fs::create_dir_all(
        repo.join("hosts/host-a/overrides/quadlet/web.container.d"),
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/host.yaml"),
        "host: host-a\nservices:\n  - web\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/overrides/quadlet/web.socket.d/20-host.conf"),
        "[Socket]\nListenStream=8080\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/overrides/quadlet/web.container.d/20-host.conf"),
        "[Service]\nEnvironment=HOST=a\n",
    )
    .unwrap();

    let output = run_migration(repo);
    assert!(
        output.status.success(),
        "migration failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Service-level drop-ins routed by kind.
    assert!(
        repo.join("services/web/systemd/web.socket.d/10-defaults.conf").exists(),
        "socket drop-in must route to systemd/ subtree"
    );
    assert!(
        repo.join("services/web/quadlet/web.container.d/10-resources.conf").exists(),
        "container drop-in must stay in quadlet/ subtree"
    );
    // Host-level drop-ins routed by kind.
    assert!(
        repo.join("hosts/host-a/web/systemd/web.socket.d/20-host.conf").exists(),
        "host socket drop-in must route to <svc>/systemd/"
    );
    assert!(
        repo.join("hosts/host-a/web/quadlet/web.container.d/20-host.conf").exists(),
        "host container drop-in must route to <svc>/quadlet/"
    );

    // Migrated tree must load cleanly through the new parser.
    let rev = git_init_commit(repo);
    let _ = load_with_host(repo, &rev, "host-a")
        .expect("post-migration tree must load with kind-aware dropin routing");
}

/// Codex P1 on PR #28 (923e728 follow-up): legacy host trees can store
/// drop-ins directly under `hosts/<h>/overrides/<unit>.<ext>.d/`
/// (spec-003 original shape) — no `quadlet/` wrapper. Earlier the
/// script only handled the `overrides/quadlet/...` shape; bare-shape
/// repos got skipped, leaving `overrides/` behind, and the new loader
/// hard-failed on the leftover legacy artifact.
#[test]
fn migration_handles_bare_overrides_dropin_shape() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    std::fs::create_dir_all(repo.join("services/web/quadlet")).unwrap();
    std::fs::write(
        repo.join("services/web/quadlet/web.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/web/quadlet/web.socket"),
        "[Socket]\nListenStream=80\n",
    )
    .unwrap();
    // Spec-003 host override layout: drop-ins directly under overrides/
    std::fs::create_dir_all(
        repo.join("hosts/host-a/overrides/web.container.d"),
    )
    .unwrap();
    std::fs::create_dir_all(
        repo.join("hosts/host-a/overrides/web.socket.d"),
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/overrides/web.container.d/20-host.conf"),
        "[Service]\nEnvironment=HOST=a\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/overrides/web.socket.d/20-host.conf"),
        "[Socket]\nListenStream=8080\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/host.yaml"),
        "host: host-a\nservices:\n  - web\n",
    )
    .unwrap();

    let output = run_migration(repo);
    assert!(
        output.status.success(),
        "migration must handle bare overrides/<unit>.d shape: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Container drop-in routes to <svc>/quadlet/, socket to <svc>/systemd/.
    assert!(
        repo.join("hosts/host-a/web/quadlet/web.container.d/20-host.conf").exists()
    );
    assert!(
        repo.join("hosts/host-a/web/systemd/web.socket.d/20-host.conf").exists()
    );
    // overrides/ scaffold removed (Codex's "leftover legacy artifact" gone).
    assert!(
        !repo.join("hosts/host-a/overrides").exists(),
        "overrides/ directory must be cleaned up after migration"
    );

    // The migrated tree must load through the new parser (proves the
    // leftover-overrides-dir issue is fixed).
    let rev = git_init_commit(repo);
    let _ = load_with_host(repo, &rev, "host-a")
        .expect("post-migration tree must load with bare-overrides shape");
}

/// Codex P2 on PR #28: the legacy ownership resolver greps
/// `^config-root: ${config_root}\b` — a `.` in `config_root` (valid
/// per the identifier regex) would act as a regex wildcard and match
/// unintended `service.yaml` values, e.g. `config-root: a.c` matches
/// `config-root: aXc`. Fix: parse service.yaml with awk and compare
/// the value as a literal string.
#[test]
fn migration_config_root_match_is_literal_not_regex() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    // Service whose config-root contains a literal char that would be a
    // regex metacharacter — `Y` here, but the bug surfaces with any
    // single char including `.`. We use `aYc` and `a.c` as the two
    // values: under the buggy grep, the host override under
    // `overrides/config/etc/a.c/` matched `aYc` (because `.` is a
    // wildcard). Post-fix: awk fixed-string compare correctly only
    // matches a literal `a.c`.
    std::fs::create_dir_all(repo.join("services/quux/quadlet")).unwrap();
    std::fs::write(
        repo.join("services/quux/quadlet/quux.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/quux/service.yaml"),
        "config-root: aYc\n",
    )
    .unwrap();
    // Host override under `a.c` (literal dot). Should NOT match `aYc`.
    std::fs::create_dir_all(
        repo.join("hosts/host-a/overrides/config/etc/a.c"),
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/overrides/config/etc/a.c/dot.toml"),
        "[dot]\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/host-a/host.yaml"),
        "host: host-a\nservices:\n  - quux\n",
    )
    .unwrap();

    let output = run_migration(repo);
    // Expected outcome: script fails because no service has config-root=a.c.
    // The pre-fix grep would have matched aYc and silently routed the
    // override to quux (wrong service).
    assert!(
        !output.status.success(),
        "ambiguous-or-no-match must fail loudly with literal compare; \
         stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no matching service"),
        "diagnostic must explain no service matched: {stderr}"
    );
}

/// Codex P2 on PR #28: when two services share the same config-root
/// AND host.yaml selects both, the host config override at
/// `overrides/config/etc/<root>/...` cannot be unambiguously routed
/// to one service. The migration script must fail loudly rather than
/// silently picking the first match.
#[test]
fn migration_phase_2b_rejects_ambiguous_config_override() {
    let tmp = TempDir::new().expect("tempdir");
    let repo = tmp.path();

    // Two services that both end up with config-root: traefik post-migration:
    // - traefik (svc-id == config-root, no service.yaml needed)
    // - traefik-dnschallenge (variant; the migration synthesizes service.yaml)
    std::fs::create_dir_all(repo.join("services/traefik/quadlet")).unwrap();
    std::fs::write(
        repo.join("services/traefik/quadlet/traefik.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::create_dir_all(repo.join("services/traefik-dnschallenge/quadlet")).unwrap();
    std::fs::create_dir_all(repo.join("services/traefik-dnschallenge/config/etc/traefik")).unwrap();
    std::fs::write(
        repo.join("services/traefik-dnschallenge/quadlet/traefik-dnschallenge.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("services/traefik-dnschallenge/config/etc/traefik/cert.toml"),
        "[certs]\n",
    )
    .unwrap();

    // Host selects BOTH services. The host override is a config file
    // under `overrides/config/etc/traefik/...`. Two candidates qualify;
    // host.yaml narrowing leaves both. The script must fail.
    std::fs::create_dir_all(repo.join("hosts/example-host/overrides/config/etc/traefik")).unwrap();
    std::fs::write(
        repo.join("hosts/example-host/host.yaml"),
        "host: example-host\nservices:\n  - traefik\n  - traefik-dnschallenge\n",
    )
    .unwrap();
    std::fs::write(
        repo.join("hosts/example-host/overrides/config/etc/traefik/traefik.toml"),
        "[ambiguous]\n",
    )
    .unwrap();

    let output = run_migration(repo);
    assert!(
        !output.status.success(),
        "ambiguous config override must fail loudly; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ambiguous") && stderr.contains("traefik"),
        "diagnostic must explain the ambiguity: {stderr}"
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
