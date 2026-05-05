//! Stateless `core-ops explain --source-repo` integration test (T022).
//!
//! Per FR-011a: explain accepts --source-repo, is read-only, requires
//! --host, writes nothing. Per spec.md SC-011 / T022 contract: this
//! test exercises **all five published examples** (one sub-test per
//! example) so SC-011's "any of the five published examples" coverage
//! is grounded in evidence rather than spot-checked.

use std::path::Path;
use std::process::Command;

fn coreops() -> Command {
    Command::new(env!("CARGO_BIN_EXE_core-ops"))
}

fn run_explain(example_slug: &str, object: &str) -> std::process::Output {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(example_slug);
    let quadlet_dir = tempfile::TempDir::new().expect("tempdir").keep();
    coreops()
        .arg("explain")
        .arg("--source-repo")
        .arg(&example_dir)
        .arg("--host")
        .arg("example")
        .arg("--quadlet-dir")
        .arg(&quadlet_dir)
        .arg(object)
        .output()
        .expect("invoke core-ops")
}

#[test]
fn stateless_explain_against_01_caddy_whoami_succeeds() {
    let output = run_explain("01-caddy-whoami", "container/caddy.container");
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("caddy"),
        "explain output should mention the inspected object: {stdout}"
    );
}

#[test]
fn stateless_explain_against_02_nextcloud_succeeds() {
    let output = run_explain("02-nextcloud", "container/nextcloud.container");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stateless_explain_against_03_immich_succeeds() {
    let output = run_explain("03-immich", "container/immich-server.container");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stateless_explain_against_04_traefik_authelia_succeeds() {
    let output = run_explain("04-traefik-authelia", "container/traefik.container");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stateless_explain_against_05_observability_succeeds() {
    let output = run_explain("05-observability", "container/prometheus.container");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stateless_explain_output_is_unaffected_by_prior_initd_state() {
    // FR-011a + clarification Q5: stateless explain must not consult
    // persisted controller state. Asserts structurally: identical
    // stateless explain invocations produce byte-identical output
    // whether `CORE_OPS_STATE_FILE` points at a populated init'd state
    // file (from an unrelated repository) or at an empty path.

    use core_ops::io::state::persist_success_state;

    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/01-caddy-whoami");
    let quadlet_dir = tempfile::TempDir::new().expect("quadlet tempdir");

    // (1) Run stateless explain with `CORE_OPS_STATE_FILE` pointing at
    //     a populated init'd state file from an unrelated repository.
    let with_state = tempfile::TempDir::new().expect("with-state tempdir");
    let state_file = with_state.path().join("status.json");
    persist_success_state(
        &state_file,
        "file:///var/lib/core-ops/some-other-repo",
        "unrelated-rev-v9",
        "feedfacefeedfacefeedfacefeedfacefeedface",
    )
    .expect("persist init'd state");
    let with_state_output = coreops()
        .arg("explain")
        .arg("--source-repo")
        .arg(&example_dir)
        .arg("--host")
        .arg("example")
        .arg("--quadlet-dir")
        .arg(quadlet_dir.path())
        .arg("container/caddy.container")
        .env("CORE_OPS_STATE_FILE", &state_file)
        .output()
        .expect("invoke core-ops with init'd state");
    assert!(
        with_state_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&with_state_output.stderr)
    );

    // (2) Run the same stateless explain with `CORE_OPS_STATE_FILE`
    //     pointing at a non-existent path (no init'd state).
    let without_state = tempfile::TempDir::new().expect("without-state tempdir");
    let absent_state_file = without_state.path().join("nonexistent.json");
    let without_state_output = coreops()
        .arg("explain")
        .arg("--source-repo")
        .arg(&example_dir)
        .arg("--host")
        .arg("example")
        .arg("--quadlet-dir")
        .arg(quadlet_dir.path())
        .arg("container/caddy.container")
        .env("CORE_OPS_STATE_FILE", &absent_state_file)
        .output()
        .expect("invoke core-ops without init'd state");
    assert!(
        without_state_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&without_state_output.stderr)
    );

    // (3) Outputs must be byte-identical — stateless explain leaks no
    //     init'd-state context.
    assert_eq!(
        with_state_output.stdout, without_state_output.stdout,
        "stateless explain output diverged based on persisted state contents"
    );
}
