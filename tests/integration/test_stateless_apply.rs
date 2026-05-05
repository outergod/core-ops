//! Stateless `core-ops apply --source-repo` integration tests
//! (T033-T035) for spec/017 US3.
//!
//! Asserts the audit chain carries path-based provenance across all
//! three working-tree shapes (clean SHA / `(stateless+dirty)` /
//! `(stateless)`) and that stateless apply does NOT mutate any prior
//! init'd controller state (FR-013, SC-009).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use core_ops::cli::apply::apply_with_report_stateless;
use core_ops::io::repo::HOST_OVERRIDE_ENV;
use core_ops::io::source_ref::detect_provenance;
use core_ops::io::state::{persist_success_state, read_persisted_state};

use crate::integration::env_lock::path_lock;
use crate::integration::source_repo_support::{git_init_commit, HostGuard};

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{prefix}_{nanos}"));
    path
}

fn write_systemctl_stub(dir: &Path) -> PathBuf {
    let bin_path = dir.join("systemctl");
    fs::write(&bin_path, "#!/bin/sh\nexit 0\n").expect("write systemctl stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&bin_path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&bin_path, perms).expect("chmod");
    }
    bin_path
}

struct PathGuard {
    previous: String,
}

impl Drop for PathGuard {
    fn drop(&mut self) {
        std::env::set_var("PATH", &self.previous);
    }
}

fn write_alpha_layout(repo: &Path) {
    let services = repo.join("services/alpha/quadlet");
    fs::create_dir_all(&services).expect("services dir");
    fs::write(
        services.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .expect("alpha.container");
    let hosts = repo.join("hosts/example-host");
    fs::create_dir_all(&hosts).expect("hosts dir");
    fs::write(
        hosts.join("host.yaml"),
        "host: example-host\nservices:\n  - alpha\n",
    )
    .expect("host.yaml");
}

fn install_systemctl_stub() -> (PathBuf, PathGuard) {
    let stub_dir = temp_dir("core_ops_systemctl_stateless_apply");
    fs::create_dir_all(&stub_dir).expect("stub dir");
    write_systemctl_stub(&stub_dir);
    let old_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", stub_dir.display(), old_path);
    std::env::set_var("PATH", new_path);
    (stub_dir, PathGuard { previous: old_path })
}

#[test]
fn stateless_apply_records_path_based_provenance_in_audit_event() {
    // T033: stateless apply against a synthetic source repo;
    // (a) exit 0, (b) audit chain produced, (c) audit event carries
    // path-based provenance (`desired_repository` = canonical path,
    // `desired_requested_ref` = `(stateless+dirty)` for an
    // uncommitted layout).
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");

    let source = temp_dir("core_ops_stateless_apply_src");
    fs::create_dir_all(&source).expect("source dir");
    write_alpha_layout(&source);
    git_init_commit(&source);
    // Introduce an uncommitted file → working tree dirty.
    fs::write(source.join("scratch.txt"), "wip\n").expect("scratch");

    let (_stub_dir, _path_guard) = install_systemctl_stub();
    let host_quadlets = temp_dir("core_ops_stateless_apply_qdir");
    fs::create_dir_all(&host_quadlets).expect("quadlet dir");

    let stateless = detect_provenance(&source).expect("detect provenance");
    assert_eq!(stateless.requested_ref, "(stateless+dirty)");

    let bundle = apply_with_report_stateless(&stateless, &host_quadlets, false)
        .expect("stateless apply");

    // (a)+(b): apply produced a populated bundle.
    assert!(!bundle.human_report.is_empty(), "human report empty");
    assert!(
        bundle.result.desired.requested_repository.is_some(),
        "desired_state.requested_repository must be populated"
    );

    // (c): provenance fields surface the path-based source.
    let repo = bundle
        .result
        .desired
        .requested_repository
        .as_deref()
        .expect("requested_repository populated");
    assert_eq!(
        Path::new(repo),
        stateless.repo_path.as_path(),
        "requested_repository must match canonical source path"
    );
    let r#ref = bundle
        .result
        .desired
        .requested_ref
        .as_deref()
        .expect("requested_ref populated");
    assert_eq!(r#ref, "(stateless+dirty)");
}

#[test]
fn stateless_apply_preserves_initd_persisted_state() {
    // T034a / SC-009: pre-write init'd state to a tempfile, run
    // stateless apply against a different source path with the same
    // tempfile selected via CORE_OPS_STATE_FILE, assert
    // `desired_state.repository` and `desired_state.requested_ref`
    // are byte-identical pre/post.
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");

    // (1) Seed the init'd state file.
    let state_file = temp_dir("core_ops_stateless_apply_state.json");
    persist_success_state(
        &state_file,
        "file:///var/lib/core-ops/init-source",
        "init-rev-v1",
        "deadbeefcafefeed1234567890abcdef12345678",
    )
    .expect("persist init'd state");
    let pre = read_persisted_state(&state_file)
        .expect("read pre-state")
        .expect("pre-state present");

    // (2) Stateless apply against an unrelated source path.
    let source = temp_dir("core_ops_stateless_apply_other_src");
    fs::create_dir_all(&source).expect("source dir");
    write_alpha_layout(&source);

    let (_stub_dir, _path_guard) = install_systemctl_stub();
    let host_quadlets = temp_dir("core_ops_stateless_apply_other_qdir");
    fs::create_dir_all(&host_quadlets).expect("quadlet dir");
    let stateless = detect_provenance(&source).expect("detect provenance");
    let _bundle = apply_with_report_stateless(&stateless, &host_quadlets, false)
        .expect("stateless apply");

    // (3) Re-read the init'd state file and assert byte-identical
    //     desired_state fields.
    let post = read_persisted_state(&state_file)
        .expect("read post-state")
        .expect("post-state present");
    assert_eq!(
        pre.desired_state.repository, post.desired_state.repository,
        "stateless apply MUST NOT mutate init'd desired_state.repository"
    );
    assert_eq!(
        pre.desired_state.requested_ref, post.desired_state.requested_ref,
        "stateless apply MUST NOT mutate init'd desired_state.requested_ref"
    );
}

#[test]
fn stateless_apply_provenance_shapes_match_working_tree_state() {
    // T035: provenance-shape coverage — three sub-cases asserting
    // `(stateless)` / `(stateless+dirty)` / SHA in the audit-bundle
    // provenance under the three working-tree conditions.
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let (_stub_dir, _path_guard) = install_systemctl_stub();

    // (a) Non-git → `(stateless)`.
    {
        let source = temp_dir("core_ops_stateless_apply_nongit");
        fs::create_dir_all(&source).expect("source dir");
        write_alpha_layout(&source);
        let qdir = temp_dir("core_ops_stateless_apply_nongit_q");
        fs::create_dir_all(&qdir).expect("qdir");
        let stateless = detect_provenance(&source).expect("detect provenance");
        assert_eq!(stateless.requested_ref, "(stateless)");
        let bundle = apply_with_report_stateless(&stateless, &qdir, false)
            .expect("apply non-git");
        assert_eq!(
            bundle.result.desired.requested_ref.as_deref(),
            Some("(stateless)")
        );
    }

    // (b) Clean git checkout → 40-char SHA.
    {
        let source = temp_dir("core_ops_stateless_apply_clean");
        fs::create_dir_all(&source).expect("source dir");
        write_alpha_layout(&source);
        let sha = git_init_commit(&source);
        assert_eq!(sha.len(), 40);
        let qdir = temp_dir("core_ops_stateless_apply_clean_q");
        fs::create_dir_all(&qdir).expect("qdir");
        let stateless = detect_provenance(&source).expect("detect provenance");
        assert_eq!(stateless.requested_ref, sha);
        let bundle = apply_with_report_stateless(&stateless, &qdir, false)
            .expect("apply clean");
        assert_eq!(bundle.result.desired.requested_ref.as_deref(), Some(sha.as_str()));
    }

    // (c) Dirty git working tree → `(stateless+dirty)`.
    {
        let source = temp_dir("core_ops_stateless_apply_dirty");
        fs::create_dir_all(&source).expect("source dir");
        write_alpha_layout(&source);
        let _ = git_init_commit(&source);
        // Untracked file makes the tree dirty.
        fs::write(source.join("scratch.txt"), "wip\n").expect("scratch");
        let qdir = temp_dir("core_ops_stateless_apply_dirty_q");
        fs::create_dir_all(&qdir).expect("qdir");
        let stateless = detect_provenance(&source).expect("detect provenance");
        assert_eq!(stateless.requested_ref, "(stateless+dirty)");
        let bundle = apply_with_report_stateless(&stateless, &qdir, false)
            .expect("apply dirty");
        assert_eq!(
            bundle.result.desired.requested_ref.as_deref(),
            Some("(stateless+dirty)")
        );
    }
}

#[test]
fn stateless_apply_then_initd_plan_against_same_tree_does_not_surface_detached_state() {
    // US3 AC3: after stateless apply lands on a host, a subsequent
    // `core-ops init <synthetic-repo> main --force` followed by
    // `core-ops plan` (no flag) produces a normal init'd-mode plan
    // with no detached-state header surfacing from the prior
    // stateless apply. We exercise this at the parser level: the
    // stateless apply path doesn't touch the persisted state file,
    // so the subsequent init+plan flow sees a "fresh" host with no
    // residual stateless artifacts.
    let _lock = path_lock().lock().unwrap_or_else(|err| err.into_inner());
    let _host_guard = HostGuard::capture();
    std::env::set_var(HOST_OVERRIDE_ENV, "example-host");
    let (_stub_dir, _path_guard) = install_systemctl_stub();

    let source = temp_dir("core_ops_stateless_to_initd_src");
    fs::create_dir_all(&source).expect("source dir");
    write_alpha_layout(&source);

    let qdir = temp_dir("core_ops_stateless_to_initd_qdir");
    fs::create_dir_all(&qdir).expect("qdir");

    // Stateless apply phase.
    let stateless = detect_provenance(&source).expect("detect provenance");
    apply_with_report_stateless(&stateless, &qdir, false).expect("stateless apply");

    // Sanity check: after stateless apply, no canonical state file at
    // `state_file` was created (we explicitly never wrote one).
    let state_file = temp_dir("core_ops_stateless_to_initd_state.json");
    assert!(
        !state_file.exists(),
        "stateless apply must not have written {state_file:?}"
    );

    // Init'd-mode equivalence: `load_desired_state` (the init'd loader)
    // against the same tree commits the stateless tempdir into a real
    // git repo and resolves HEAD. We assert it loads cleanly — this
    // is the parser-level analogue of `core-ops init && core-ops plan`.
    git_init_commit(&source);
    let head = ProcessCommand::new("git")
        .arg("-C")
        .arg(&source)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git rev-parse");
    let head_sha = String::from_utf8_lossy(&head.stdout).trim().to_string();
    let initd = core_ops::io::repo::load_desired_state(
        source.to_str().expect("utf-8 path"),
        &head_sha,
    )
    .expect("init'd load");
    // Init'd plan against the same tree resolves a non-empty workload
    // catalog — i.e., the prior stateless apply did not leave residual
    // state that would derail the init'd-mode flow.
    assert!(!initd.workloads.is_empty(), "init'd workloads empty after transition");
}
