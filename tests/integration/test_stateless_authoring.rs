//! Stateless `--source-repo` authoring + iteration tests for US2
//! (T030, T031). Validates that an operator can copy an example to a
//! scratch directory, rename hosts, edit configs, and re-run plan
//! without ever running `core-ops init` or `git init`. Then validates
//! that the stateless-to-init'd transition produces an equivalent
//! plan against the same source tree (US2 AC3).

use std::path::Path;
use std::process::Command;

use core_ops::io::repo::HOST_OVERRIDE_ENV;

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{copy_dir_recursive, git_init_commit, HostGuard};

fn coreops() -> Command {
    Command::new(env!("CARGO_BIN_EXE_core-ops"))
}

#[test]
fn copy_example_rename_host_and_iterate_succeeds() {
    // T030 / US2 AC1: copy `examples/02-nextcloud/` to a scratch
    // tempdir (no git init), rename `hosts/example/` to `hosts/myhost/`,
    // edit `host.yaml`, run `core-ops plan --source-repo <scratch>
    // --host myhost` → exit 0.
    let scratch = tempfile::TempDir::new().expect("scratch tempdir");
    let example_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/02-nextcloud");
    copy_dir_recursive(&example_src, scratch.path()).expect("copy example");

    // Rename hosts/example/ → hosts/myhost/.
    let old_host = scratch.path().join("hosts/example");
    let new_host = scratch.path().join("hosts/myhost");
    std::fs::rename(&old_host, &new_host).expect("rename host dir");
    // Update host.yaml's `host:` field to match the directory name
    // (deny_unknown_fields hardening rejects mismatches).
    let host_yaml = new_host.join("host.yaml");
    let body = std::fs::read_to_string(&host_yaml).expect("read host.yaml");
    let rewritten = body.replace("host: example", "host: myhost");
    std::fs::write(&host_yaml, rewritten).expect("write host.yaml");

    let qdir = tempfile::TempDir::new().expect("quadlet tempdir");
    let output = coreops()
        .arg("plan")
        .arg("--source-repo")
        .arg(scratch.path())
        .arg("--host")
        .arg("myhost")
        .arg("--quadlet-dir")
        .arg(qdir.path())
        .output()
        .expect("invoke core-ops");
    assert!(
        output.status.success(),
        "scratch-dir plan after host-rename should exit 0\nstderr:\n{}\nstdout:\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("(stateless)"),
        "non-git scratch dir must surface (stateless) sentinel: {stdout}"
    );
}

#[test]
fn stateless_and_initd_plans_against_same_tree_are_equivalent() {
    // T031 / US2 AC3: stateless plan against a scratch dir, then
    // `git init && core-ops init && core-ops plan` (no --source-repo),
    // assert the two plans produce equivalent action sets via the
    // workload list comparison. We use the library API for the
    // comparison so both modes share an identical assertion surface.

    use core_ops::io::repo::{load_desired_state, load_desired_state_from_path};

    let scratch = tempfile::TempDir::new().expect("scratch tempdir");
    let example_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/01-caddy-whoami");
    copy_dir_recursive(&example_src, scratch.path()).expect("copy example");

    // Run both loads under the path_lock + same CORE_OPS_HOST so the
    // host resolver sees `example` for both.
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example");

    // Stateless load: scratch dir as-is, no git, sentinel ref.
    let stateless = load_desired_state_from_path(
        scratch.path(),
        scratch.path().to_str().expect("utf-8 path"),
        "(stateless)",
    )
    .expect("stateless load");

    // Init'd-mode load: git_init_commit creates a real revision and
    // `load_desired_state` clones into a tempdir + checks out HEAD.
    let rev = git_init_commit(scratch.path());
    let initd = load_desired_state(
        scratch.path().to_str().expect("utf-8 path"),
        &rev,
    )
    .expect("init'd load");

    // Equivalence: same workload set (by systemd_unit_name), same
    // managed_config_paths, same managed_config_roots. Provenance
    // fields legitimately differ between modes — that's the point of
    // the value-level conventions in data-model.md E1.
    let stateless_units: Vec<String> = stateless
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.clone())
        .collect();
    let initd_units: Vec<String> = initd
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.clone())
        .collect();
    assert_eq!(
        stateless_units, initd_units,
        "stateless vs init'd workload sets diverged: {stateless_units:?} != {initd_units:?}"
    );
    assert_eq!(stateless.managed_config_paths, initd.managed_config_paths);
    assert_eq!(stateless.managed_config_roots, initd.managed_config_roots);
}
