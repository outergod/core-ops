use core_ops::core::types::DesiredState;
use core_ops::io::repo::RepoError;

use crate::integration::source_repo_support::{
    git_init_commit, load_with_host, materialize_example, materialize_skeleton, write_host_yaml,
};

fn unit_names(state: &DesiredState) -> Vec<String> {
    state
        .workloads
        .iter()
        .map(|w| w.systemd_unit_name.clone())
        .collect()
}

// ---- Example load tests (FR-001..FR-008, happy path) ----

#[test]
fn example_01_minimal_single_service_loads() {
    let (repo, rev) = materialize_example("01-minimal-single-service");
    let state = load_with_host(repo.path(), &rev, "example-host").expect("load");
    let names = unit_names(&state);
    assert!(
        names.iter().any(|n| n == "whoami.container"),
        "missing whoami.container: {names:?}"
    );
    assert!(
        state
            .managed_config_paths
            .contains(&"/etc/whoami/whoami.toml".to_string()),
        "missing /etc/whoami/whoami.toml: {:?}",
        state.managed_config_paths
    );
    assert_eq!(
        state.managed_config_roots,
        vec!["/etc/whoami".to_string()]
    );
}

#[test]
fn example_02_variant_config_root_loads() {
    let (repo, rev) = materialize_example("02-variant-config-root");
    let state = load_with_host(repo.path(), &rev, "example-host").expect("load");
    let names = unit_names(&state);
    assert!(
        names.iter().any(|n| n == "traefik-dnschallenge.container"),
        "missing container: {names:?}"
    );
    // Service id is `traefik-dnschallenge` but config-root is `traefik`.
    assert!(
        state
            .managed_config_paths
            .contains(&"/etc/traefik/traefik.yaml".to_string()),
        "config not rooted under /etc/traefik/: {:?}",
        state.managed_config_paths
    );
    assert_eq!(state.managed_config_roots, vec!["/etc/traefik".to_string()]);
}

#[test]
fn example_03_multi_unit_with_dropins_loads() {
    let (repo, rev) = materialize_example("03-multi-unit-with-dropins");
    let state = load_with_host(repo.path(), &rev, "example-host").expect("load");
    let names = unit_names(&state);
    assert!(
        names.iter().any(|n| n == "webhook-receiver.container"),
        "missing container unit: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "webhook-receiver.socket"),
        "missing socket unit: {names:?}"
    );
}

#[test]
fn example_04_host_overlay_loads() {
    let (repo, rev) = materialize_example("04-host-overlay");
    let state = load_with_host(repo.path(), &rev, "host-a").expect("load");
    let names = unit_names(&state);
    assert!(
        names.iter().any(|n| n == "node-exporter.container"),
        "missing container unit: {names:?}"
    );
    // The host's config/ provides a whole-file replacement; the resolved
    // destination is still `/etc/node-exporter/node-exporter.env` (FR-010
    // invariant: rooted under /etc/<config-root>/).
    assert!(
        state
            .managed_config_paths
            .contains(&"/etc/node-exporter/node-exporter.env".to_string()),
        "missing config target: {:?}",
        state.managed_config_paths
    );
}

// ---- Full systemd unit extension set (.timer / .target / .path) ----
//
// `contracts/layout.md` and SKILL.md §5 advertise that
// `services/<svc>/systemd/` accepts `.socket`, `.timer`, `.target`,
// `.mount`, and `.path`. This test guards the contract: a service
// whose `systemd/` tree contains a timer, a target, and a path unit
// loads cleanly and surfaces all three as workloads. Codex-flagged
// regression on PR #28.
#[test]
fn systemd_payload_accepts_timer_target_and_path_extensions() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc_quadlet = services.join("scheduler/quadlet");
    let svc_systemd = services.join("scheduler/systemd");
    std::fs::create_dir_all(&svc_quadlet).unwrap();
    std::fs::create_dir_all(&svc_systemd).unwrap();
    std::fs::write(
        svc_quadlet.join("scheduler.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        svc_systemd.join("scheduler.timer"),
        "[Timer]\nOnCalendar=hourly\n[Install]\nWantedBy=timers.target\n",
    )
    .unwrap();
    std::fs::write(
        svc_systemd.join("scheduler.target"),
        "[Unit]\nDescription=scheduler readiness target\n",
    )
    .unwrap();
    std::fs::write(
        svc_systemd.join("scheduler.path"),
        "[Path]\nPathChanged=/var/lib/scheduler/inbox\n[Install]\nWantedBy=paths.target\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["scheduler"]);
    let rev = git_init_commit(tmp.path());

    let state = load_with_host(tmp.path(), &rev, "example-host").expect("load");
    let names = unit_names(&state);
    for unit in [
        "scheduler.container",
        "scheduler.timer",
        "scheduler.target",
        "scheduler.path",
    ] {
        assert!(
            names.iter().any(|n| n == unit),
            "missing {unit} in {names:?}"
        );
    }
}

// ---- Codex P2 #1: reject unrecognized non-.d directories in payload trees ----
//
// A typo like `quadlet/foo.container.dropin/` would silently drop
// drop-ins on the floor. Strict-layout contract demands fail-fast.
#[test]
fn unrecognized_non_dropin_directory_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Typo: `dropin` instead of `.d`. Should be rejected, not skipped.
    std::fs::create_dir_all(svc.join("quadlet/alpha.container.dropin")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container.dropin/10-resources.conf"),
        "[Service]\nMemoryMax=256M\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected rejection");
    assert!(
        matches!(err, RepoError::LegacyArtifact(ref p) if p.ends_with("alpha.container.dropin")),
        "expected LegacyArtifact pointing at the typo, got {err:?}"
    );
}

// ---- Codex P2 #2: literal `etc/` subdir under config/ is legitimate ----
//
// FR-002 says `config/<rel>` is generic. A user wanting destination
// `/etc/<config-root>/etc/foo/bar` puts the source at
// `services/<svc>/config/etc/foo/bar`. The parser previously rejected
// any `config/etc/` as legacy; the legacy mirror pattern is detected
// elsewhere (top-level quadlets/, quadlet-overrides/, hosts/<h>/overrides/).
#[test]
fn literal_etc_subdir_under_config_is_legitimate() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::create_dir_all(svc.join("config/etc/dnsmasq.d")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(
        svc.join("config/etc/dnsmasq.d/10-upstream.conf"),
        "server=1.1.1.1\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let state = load_with_host(tmp.path(), &rev, "example-host").expect("load");
    assert!(
        state
            .managed_config_paths
            .contains(&"/etc/alpha/etc/dnsmasq.d/10-upstream.conf".to_string()),
        "expected /etc/alpha/etc/dnsmasq.d/10-upstream.conf in {:?}",
        state.managed_config_paths
    );
}

// ---- Codex P1 #2: host overlay must reference a service host.yaml selects ----
//
// Without this check, a typo like `hosts/<h>/traefic-dnschallenge/`
// (note: missing 'k') would silently attach drop-ins via raw unit-name
// matching, causing cross-service drift instead of failing fast.
#[test]
fn host_overlay_for_unselected_service_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Real service: traefik-dnschallenge.
    let svc = services.join("traefik-dnschallenge");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/traefik-dnschallenge.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Host selects the real service.
    write_host_yaml(&hosts, "example-host", &["traefik-dnschallenge"]);
    // Operator typo: directory named `traefic-dnschallenge` (missing 'k').
    // It happens to contain a drop-in whose target unit name DOES exist
    // (so the previous behaviour silently applied the drop-in).
    let typo_overlay = hosts.join(
        "example-host/traefic-dnschallenge/quadlet/traefik-dnschallenge.container.d",
    );
    std::fs::create_dir_all(&typo_overlay).unwrap();
    std::fs::write(
        typo_overlay.join("10-typo.conf"),
        "[Service]\nMemoryMax=128M\n",
    )
    .unwrap();
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected host overlay typo rejection");
    let msg = err.to_string();
    assert!(
        msg.contains("traefic-dnschallenge")
            && msg.contains("host.yaml does not select"),
        "diagnostic must name the typo and explain the rule: {msg}"
    );
}

// ---- Codex P1 #3: enforce full identifier regex ----
//
// Without the full pattern check, a `config-root: foo/bar` in
// service.yaml would create destinations like `/etc/foo/bar/...`
// while observed-state scans collapsed the managed root to
// `/etc/foo`, flagging unrelated files for removal. Identifiers
// must match `[A-Za-z0-9][A-Za-z0-9._-]*`.
#[test]
fn config_root_with_slash_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(svc.join("service.yaml"), "config-root: foo/bar\n").unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected rejection");
    assert!(
        matches!(err, RepoError::InvalidIdentifier(ref name) if name == "foo/bar"),
        "expected InvalidIdentifier(foo/bar), got {err:?}"
    );
    assert!(
        err.to_string().contains("[A-Za-z0-9][A-Za-z0-9._-]*"),
        "diagnostic must surface the documented pattern: {err}"
    );
}

#[test]
fn service_id_with_space_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("bad name");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/bad.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["bad name"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected rejection");
    assert!(
        matches!(err, RepoError::InvalidIdentifier(ref name) if name == "bad name"),
        "expected InvalidIdentifier(\"bad name\"), got {err:?}"
    );
}

// ---- Codex P2 #3: drop-in target extension must match payload kind ----
//
// `services/<svc>/systemd/api.container.d/` is rejected: the target
// unit `api.container` is a Quadlet kind, not a Systemd-payload kind.
// Without this cross-check, a cross-kind typo silently attaches to a
// real `quadlet/api.container` if it happens to exist.
#[test]
fn dropin_target_kind_mismatched_with_payload_subtree_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::create_dir_all(svc.join("systemd/api.container.d")).unwrap();
    std::fs::write(
        svc.join("quadlet/api.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Cross-kind typo: container drop-in placed under systemd/.
    std::fs::write(
        svc.join("systemd/api.container.d/10-resources.conf"),
        "[Service]\nMemoryMax=256M\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected rejection");
    assert!(
        matches!(
            err,
            RepoError::InvalidPayloadKindFile { kind: "systemd", .. }
        ),
        "expected InvalidPayloadKindFile(kind=systemd), got {err:?}"
    );
}

// ---- Codex P2 #4: host overlay drop-in target kind cross-check ----
//
// Same fix as `dropin_target_kind_mismatched_with_payload_subtree_rejected`
// but for the host overlay path (walk_host_service_overlay). A typo
// `hosts/<h>/<svc>/systemd/api.container.d/` is rejected — container
// drop-ins under a systemd subtree would otherwise silently override
// a real `quadlet/api.container` if it exists.
#[test]
fn host_overlay_dropin_target_kind_mismatch_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/api.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    // Cross-kind typo: container drop-in under systemd/ in the host
    // overlay tree.
    let overlay_systemd =
        hosts.join("example-host/alpha/systemd/api.container.d");
    std::fs::create_dir_all(&overlay_systemd).unwrap();
    std::fs::write(
        overlay_systemd.join("20-host.conf"),
        "[Service]\nMemoryMax=128M\n",
    )
    .unwrap();
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected rejection");
    assert!(
        matches!(
            err,
            RepoError::InvalidPayloadKindFile { kind: "systemd", .. }
        ),
        "expected InvalidPayloadKindFile(kind=systemd), got {err:?}"
    );
}

// ---- Codex P2 #5: host identifiers must satisfy the same id rules ----
//
// Host identifiers are subject to FR-009 + the identifier pattern just
// like service ids and config-roots. Without this, hosts/_metadata/ or
// hosts/foo bar/ would silently load.
#[test]
fn host_id_with_reserved_prefix_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "_metadata", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "_metadata").expect_err("expected rejection");
    assert!(
        matches!(err, RepoError::ReservedName(ref name) if name == "_metadata"),
        "expected ReservedName(_metadata), got {err:?}"
    );
}

#[test]
fn host_id_with_invalid_chars_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "host with space", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "host with space")
        .expect_err("expected rejection");
    assert!(
        matches!(err, RepoError::InvalidIdentifier(ref name) if name == "host with space"),
        "expected InvalidIdentifier(\"host with space\"), got {err:?}"
    );
}

// ---- Codex P2 #6: unrecognized top-level directories rejected ----
//
// contracts/layout.md says repo-root directories are exactly
// `services/` and `hosts/`, plus anything in the reserved `_*` /
// `.*` namespace. Without this, a typo like `<repo>/servcies/`
// silently drops the operator's intended services tree.
#[test]
fn typoed_top_level_directory_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Real intended layout (would parse cleanly on its own).
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    // Operator typo at the repo root.
    let typo = tmp.path().join("servcies");
    std::fs::create_dir_all(typo.join("beta/quadlet")).unwrap();
    std::fs::write(
        typo.join("beta/quadlet/beta.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected typo rejection");
    assert!(
        matches!(err, RepoError::LegacyArtifact(ref p) if p.ends_with("servcies")),
        "expected LegacyArtifact(servcies), got {err:?}"
    );
}

#[test]
fn top_level_files_and_reserved_dirs_are_tolerated() {
    // Symmetry guard: README.md / LICENSE / CHANGELOG (regular
    // files) plus `_local/` and `.git/` (reserved-prefix dirs) must
    // still load cleanly. Tests that the strictness in
    // typoed_top_level_directory_rejected isn't over-applied.
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    // Tolerated regular files at root.
    std::fs::write(tmp.path().join("README.md"), "# example\n").unwrap();
    std::fs::write(tmp.path().join("LICENSE"), "AGPL-3.0\n").unwrap();
    // Tolerated reserved-prefix dirs.
    std::fs::create_dir_all(tmp.path().join("_local")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".cache")).unwrap();
    let rev = git_init_commit(tmp.path());

    load_with_host(tmp.path(), &rev, "example-host")
        .expect("regular files and reserved-prefix dirs at repo root must be tolerated");
}

// ---- FR-009: reserved-name rejection ----

#[test]
fn reserved_service_name_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("_admin");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/admin.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["_admin"]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected reserved name rejection");
    assert!(
        matches!(err, RepoError::ReservedName(ref name) if name == "_admin"),
        "expected ReservedName(_admin), got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("reserved name") && msg.contains("_admin"),
        "diagnostic missing reserved-name pointer: {msg}"
    );
}

// ---- FR-010: config destinations stay rooted under /etc/<config-root>/ ----
//
// The literal `..` defense in `read_config_files` is unreachable through a
// standard filesystem walk (`walk_config_dir` only descends, never produces
// `..` components). This test exercises the integration-level invariant
// FR-010 protects: nested `config/` files resolve to deterministic paths
// rooted under `/etc/<config-root>/`. A bug that lets a path escape the
// root would surface here as a non-`/etc/<config-root>/` target.

#[test]
fn nested_config_paths_stay_rooted_under_config_root() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc_cfg = services.join("nested/config/sub/dir");
    std::fs::create_dir_all(&svc_cfg).unwrap();
    std::fs::write(svc_cfg.join("file.toml"), "key = 1\n").unwrap();
    let svc_quad = services.join("nested/quadlet");
    std::fs::create_dir_all(&svc_quad).unwrap();
    std::fs::write(
        svc_quad.join("nested.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["nested"]);
    let rev = git_init_commit(tmp.path());

    let state = load_with_host(tmp.path(), &rev, "example-host").expect("load");
    let target = "/etc/nested/sub/dir/file.toml";
    assert!(
        state.managed_config_paths.iter().any(|p| p == target),
        "expected rooted target {target}, got {:?}",
        state.managed_config_paths
    );
    for path in &state.managed_config_paths {
        assert!(
            path.starts_with("/etc/nested/"),
            "path escaped /etc/nested/: {path}"
        );
    }
}

// ---- FR-011: destination conflict ----

#[test]
fn destination_conflict_across_services_rejected() {
    // Two services with overlapping config-root produce two files mapping
    // to the same /etc/<config-root>/<rel> destination. FR-011 requires
    // load-time rejection.
    let (tmp, services, hosts) = materialize_skeleton();

    // service `alpha` with config-root `shared`
    let alpha = services.join("alpha");
    std::fs::create_dir_all(alpha.join("quadlet")).unwrap();
    std::fs::create_dir_all(alpha.join("config")).unwrap();
    std::fs::write(alpha.join("service.yaml"), "config-root: shared\n").unwrap();
    std::fs::write(
        alpha.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(alpha.join("config/conflict.toml"), "from = \"alpha\"\n").unwrap();

    // service `beta` with config-root `shared` (collides on conflict.toml)
    let beta = services.join("beta");
    std::fs::create_dir_all(beta.join("quadlet")).unwrap();
    std::fs::create_dir_all(beta.join("config")).unwrap();
    std::fs::write(beta.join("service.yaml"), "config-root: shared\n").unwrap();
    std::fs::write(
        beta.join("quadlet/beta.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    std::fs::write(beta.join("config/conflict.toml"), "from = \"beta\"\n").unwrap();

    write_host_yaml(&hosts, "example-host", &["alpha", "beta"]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected destination conflict rejection");
    let msg = err.to_string();
    assert!(
        msg.contains("/etc/shared/conflict.toml") || msg.to_lowercase().contains("duplicate"),
        "diagnostic missing conflict pointer: {msg}"
    );
}

// ---- FR-012: legacy artifact rejection ----

#[test]
fn legacy_top_level_quadlets_dir_rejected() {
    let (tmp, _services, _hosts) = materialize_skeleton();
    let legacy = tmp.path().join("quadlets");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(
        legacy.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&_hosts, "example-host", &[]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected legacy-artifact rejection");
    assert!(
        matches!(err, RepoError::LegacyArtifact(ref p) if p.ends_with("quadlets")),
        "expected LegacyArtifact(quadlets), got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("legacy layout artifact") && msg.contains("migrate"),
        "diagnostic missing migration pointer: {msg}"
    );
}

#[test]
fn legacy_quadlet_overrides_dir_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet-overrides/alpha.container.d")).unwrap();
    std::fs::write(
        svc.join("quadlet-overrides/alpha.container.d/10-resources.conf"),
        "[Service]\nMemoryMax=256M\n",
    )
    .unwrap();
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected legacy quadlet-overrides rejection");
    assert!(
        matches!(err, RepoError::LegacyArtifact(ref p) if p.ends_with("quadlet-overrides")),
        "expected LegacyArtifact(quadlet-overrides), got {err:?}"
    );
}

// ---- FR-013: orphan drop-in ----

#[test]
fn orphan_dropin_rejected() {
    // A drop-in for a unit that does not exist in the merged set must
    // produce a diagnostic. We park the orphan drop-in inside `quadlet/`
    // so the parser walks it via `read_payload_dropins`, then validation
    // surfaces no matching parent unit.
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet/missing.container.d")).unwrap();
    std::fs::write(
        svc.join("quadlet/missing.container.d/10-resources.conf"),
        "[Service]\nMemoryMax=256M\n",
    )
    .unwrap();
    // Provide a real container so the service itself is non-empty.
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected orphan drop-in");
    let msg = err.to_string();
    assert!(
        msg.contains("missing.container") || msg.contains("orphan"),
        "diagnostic missing parent-unit pointer: {msg}"
    );
}

// ---- FR-014/FR-015: deterministic / idempotent parsing ----

#[test]
fn repeated_load_yields_identical_workloads() {
    let (repo, rev) = materialize_example("03-multi-unit-with-dropins");
    let first =
        load_with_host(repo.path(), &rev, "example-host").expect("load first");
    let second =
        load_with_host(repo.path(), &rev, "example-host").expect("load second");

    // `repository_ref` and the internal `_repo_temp` paths differ between
    // calls (each call clones into a fresh TempDir). Compare every other
    // field that drives the planner.
    assert_eq!(first.revision_id, second.revision_id);
    assert_eq!(first.requested_repository, second.requested_repository);
    assert_eq!(first.requested_ref, second.requested_ref);
    assert_eq!(first.workloads, second.workloads);
    assert_eq!(first.mount_declarations, second.mount_declarations);
    assert_eq!(first.mount_dependencies, second.mount_dependencies);
    assert_eq!(first.managed_config_paths, second.managed_config_paths);
    assert_eq!(first.managed_config_roots, second.managed_config_roots);
    assert_eq!(first.invariants, second.invariants);
    assert_eq!(first.boundaries, second.boundaries);
}

// ---- FR-016: missing service diagnostic ----

#[test]
fn host_selects_missing_service_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Only `alpha` exists in the catalog.
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Host selects `alpha` and a non-existent `phantom`.
    write_host_yaml(&hosts, "example-host", &["alpha", "phantom"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected missing service");
    let msg = err.to_string();
    assert!(
        msg.contains("phantom"),
        "diagnostic missing service id 'phantom': {msg}"
    );
    assert!(
        msg.contains("example-host") || msg.contains("host"),
        "diagnostic missing host context: {msg}"
    );
}

// ---- FR-017: malformed service.yaml diagnostics ----

#[test]
fn service_yaml_unknown_key_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Unknown key `bogus-field` (deny_unknown_fields on ServiceManifest).
    std::fs::write(
        svc.join("service.yaml"),
        "config-root: alpha\nbogus-field: oops\n",
    )
    .unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected unknown-key error");
    assert!(
        matches!(err, RepoError::InvalidServiceManifest(_)),
        "expected InvalidServiceManifest, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("service.yaml"),
        "diagnostic missing file pointer: {msg}"
    );
    assert!(
        msg.contains("bogus-field") || msg.to_lowercase().contains("unknown"),
        "diagnostic missing offending key: {msg}"
    );
}

#[test]
fn service_yaml_parse_error_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Syntactically invalid YAML.
    std::fs::write(svc.join("service.yaml"), "config-root: [unterminated\n").unwrap();
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err =
        load_with_host(tmp.path(), &rev, "example-host").expect_err("expected parse error");
    assert!(
        matches!(err, RepoError::InvalidServiceManifest(_)),
        "expected InvalidServiceManifest, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("service.yaml"),
        "diagnostic missing file pointer: {msg}"
    );
}

// ---- FR-018: host overlay base-unit rejection ----

#[test]
fn host_overlay_base_unit_rejected() {
    let (tmp, services, hosts) = materialize_skeleton();
    // Base service must exist.
    let svc = services.join("alpha");
    std::fs::create_dir_all(svc.join("quadlet")).unwrap();
    std::fs::write(
        svc.join("quadlet/alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Host overlay tries to introduce a base unit (a *.container directly
    // under hosts/<h>/<svc>/quadlet/, not nested in a *.d/ drop-in dir).
    let overlay = hosts.join("example-host/alpha/quadlet");
    std::fs::create_dir_all(&overlay).unwrap();
    std::fs::write(
        overlay.join("alpha.container"),
        "[Container]\nImage=alpine\n",
    )
    .unwrap();
    // Host needs host.yaml; the alpha/ sibling dir is the overlay subject.
    write_host_yaml(&hosts, "example-host", &["alpha"]);
    let rev = git_init_commit(tmp.path());

    let err = load_with_host(tmp.path(), &rev, "example-host")
        .expect_err("expected host overlay base-unit rejection");
    assert!(
        matches!(err, RepoError::HostOverlayBaseUnit(_)),
        "expected HostOverlayBaseUnit, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("base unit") && msg.contains("drop-ins"),
        "diagnostic missing FR-018 wording: {msg}"
    );
}
