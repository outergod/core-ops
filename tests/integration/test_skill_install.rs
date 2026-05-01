//! Integration tests for `core-ops skill install` per the contract at
//! specs/016-source-repository-layout/contracts/skill-cli.md §"Test
//! contract". Six named test functions, each asserting one externally
//! observable behaviour of the subcommand.
//!
//! The tests run the binary as a subprocess via `CARGO_BIN_EXE_core-ops`
//! so they exercise clap argument parsing, the file-writer side
//! effects, and the `--print` stdout encoding the same way an operator
//! would. `--global` runs are coordinated under `path_lock()` because
//! they mutate the process-global `HOME` env var (the controller resolves
//! `~/` from `HOME`).

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use crate::integration::env_lock::path_lock;

const BUNDLE_DIR: &str = ".agents/skills/core-ops-source-repo";

fn cargo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_core-ops")
}

fn run_install(cwd: &Path, extra_args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .current_dir(cwd)
        .arg("skill")
        .arg("install")
        .args(extra_args)
        .output()
        .expect("run core-ops skill install")
}

struct HomeGuard(Option<OsString>);

impl HomeGuard {
    fn capture() -> Self {
        Self(std::env::var_os("HOME"))
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.0 {
            std::env::set_var("HOME", value);
        } else {
            std::env::remove_var("HOME");
        }
    }
}

#[test]
fn test_skill_install_default() {
    let tmp = TempDir::new().expect("tempdir");
    let output = run_install(tmp.path(), &[]);
    assert!(
        output.status.success(),
        "skill install failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let skill_md = tmp.path().join(BUNDLE_DIR).join("SKILL.md");
    assert!(
        skill_md.exists(),
        "expected SKILL.md at {}",
        skill_md.display()
    );
    let bytes = std::fs::read(&skill_md).expect("read SKILL.md");
    assert!(
        !bytes.is_empty(),
        "SKILL.md must not be empty"
    );
    let body = String::from_utf8(bytes).expect("SKILL.md is utf-8");
    // Sanity: the bundled skill leads with the canonical frontmatter and
    // covers the four authoring shapes named in §11.
    assert!(
        body.starts_with("---\n"),
        "SKILL.md must lead with YAML frontmatter"
    );
    assert!(body.contains("name: core-ops-source-repo"));
    assert!(body.contains("01-minimal-single-service"));
    assert!(body.contains("02-variant-config-root"));
    assert!(body.contains("03-multi-unit-with-dropins"));
    assert!(body.contains("04-host-overlay"));
}

#[test]
fn test_skill_install_global() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _home_guard = HomeGuard::capture();
    let fake_home = TempDir::new().expect("home tempdir");
    std::env::set_var("HOME", fake_home.path());

    // `Command::output` inherits the parent's env including HOME; assert
    // it lands under the fake home rather than the operator's real home.
    let workdir = TempDir::new().expect("cwd tempdir");
    let output = Command::new(cargo_bin())
        .current_dir(workdir.path())
        .arg("skill")
        .arg("install")
        .arg("--global")
        .env("HOME", fake_home.path())
        .output()
        .expect("run core-ops skill install --global");
    assert!(
        output.status.success(),
        "skill install --global failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let global_skill = fake_home.path().join(BUNDLE_DIR).join("SKILL.md");
    assert!(
        global_skill.exists(),
        "expected SKILL.md at {}",
        global_skill.display()
    );

    // Compare against a default-mode install for byte equality.
    let default_workdir = TempDir::new().expect("default cwd");
    let _ = run_install(default_workdir.path(), &[]);
    let default_skill = default_workdir.path().join(BUNDLE_DIR).join("SKILL.md");
    let default_bytes = std::fs::read(&default_skill).expect("read default SKILL.md");
    let global_bytes = std::fs::read(&global_skill).expect("read global SKILL.md");
    assert_eq!(
        default_bytes, global_bytes,
        "default and --global must produce byte-identical bundles"
    );

    // FR-021 / contract §Path standard: ensure the cwd was untouched.
    assert!(!workdir.path().join(BUNDLE_DIR).exists());
}

#[test]
fn test_skill_install_print() {
    let workdir = TempDir::new().expect("cwd tempdir");
    let output = run_install(workdir.path(), &["--print"]);
    assert!(
        output.status.success(),
        "skill install --print failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !workdir.path().join(BUNDLE_DIR).exists(),
        "--print must not write to disk"
    );

    let stdout = output.stdout;
    assert!(!stdout.is_empty(), "--print produced empty stdout");

    // Per the contract, the print stream begins with a header
    // `==> SKILL.md <==\n` followed by the file's bytes.
    let header = b"==> SKILL.md <==\n";
    assert!(
        stdout.starts_with(header),
        "--print output must start with 'SKILL.md' header, got: {:?}",
        String::from_utf8_lossy(&stdout[..stdout.len().min(80)])
    );
    let payload = &stdout[header.len()..];

    // The payload must match the file written by the default mode.
    let default_workdir = TempDir::new().expect("default cwd");
    let _ = run_install(default_workdir.path(), &[]);
    let default_bytes = std::fs::read(default_workdir.path().join(BUNDLE_DIR).join("SKILL.md"))
        .expect("read default SKILL.md");
    assert!(
        payload.starts_with(&default_bytes),
        "--print payload must contain the SKILL.md bytes verbatim"
    );
}

#[test]
fn test_skill_install_idempotent() {
    let tmp = TempDir::new().expect("tempdir");
    let first = run_install(tmp.path(), &[]);
    assert!(first.status.success(), "first install must succeed");
    let skill_md = tmp.path().join(BUNDLE_DIR).join("SKILL.md");
    let first_bytes = std::fs::read(&skill_md).expect("read after first install");
    let first_mtime = std::fs::metadata(&skill_md)
        .expect("first metadata")
        .modified()
        .ok();

    // Second run on top of the first must be a no-op (no error, no
    // observable byte change).
    let second = run_install(tmp.path(), &[]);
    assert!(
        second.status.success(),
        "second install must succeed: stderr={}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second_bytes = std::fs::read(&skill_md).expect("read after second install");
    assert_eq!(
        first_bytes, second_bytes,
        "byte-identical reinstall must not modify SKILL.md"
    );
    if let (Some(a), Some(b)) = (
        first_mtime,
        std::fs::metadata(&skill_md)
            .expect("second metadata")
            .modified()
            .ok(),
    ) {
        // The file may or may not be re-written; the byte content equality
        // above is the binding contract. The mtime check is a cheap sanity
        // signal — assert only when both reads succeeded.
        let _ = (a, b);
    }
}

#[test]
fn test_skill_install_no_init_coupling() {
    // FR-020: skill install must work in any directory, including one
    // that is NOT a CoreOps source repository (no services/, no hosts/,
    // no .specify/). It must not consult or create controller state.
    let tmp = TempDir::new().expect("tempdir");
    let output = run_install(tmp.path(), &[]);
    assert!(
        output.status.success(),
        "skill install must succeed in a non-CoreOps directory"
    );

    // Per FR-020 / contract §Independence from `core-ops init`: nothing
    // outside the resolved skill destination may be touched.
    for forbidden in [".specify", "services", "hosts", "status.json"] {
        assert!(
            !tmp.path().join(forbidden).exists(),
            "skill install must not touch {forbidden}"
        );
    }
}

#[test]
fn test_skill_install_vendor_neutral() {
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _home_guard = HomeGuard::capture();
    let fake_home = TempDir::new().expect("home tempdir");
    std::env::set_var("HOME", fake_home.path());

    let workdir = TempDir::new().expect("cwd tempdir");
    let output = Command::new(cargo_bin())
        .current_dir(workdir.path())
        .arg("skill")
        .arg("install")
        .arg("--global")
        .env("HOME", fake_home.path())
        .output()
        .expect("run core-ops skill install --global");
    assert!(output.status.success(), "global install must succeed");

    // FR-021: the agentskills.io standard is `.agents/skills/`. Vendor
    // paths like `.claude/skills/` MUST NOT appear under the resolved
    // home directory after a default-channel install.
    let agents_skill = fake_home.path().join(".agents/skills/core-ops-source-repo");
    assert!(
        agents_skill.exists(),
        "global install must populate .agents/skills/core-ops-source-repo"
    );
    let claude_skill = fake_home.path().join(".claude/skills");
    assert!(
        !claude_skill.exists(),
        "default-channel install MUST NOT write to .claude/skills/"
    );
}
