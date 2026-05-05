---

description: "Task list for spec 016: source repository layout formalization"
---

# Tasks: Source Repository Layout Formalization

**Input**: Design documents in `/specs/016-source-repository-layout/`
**Prerequisites**: spec.md ✓, plan.md ✓, research.md ✓, data-model.md ✓, contracts/ ✓, quickstart.md ✓

**Tests**: REQUIRED. Per constitution principle 10, every Rust change ships with `cargo test` and `cargo clippy --all-targets -- -D warnings` green. This feature also requires a VM-backed scenario for the externally visible config-root semantics (T106).

**Organization**: One phase per user story (P1 → P2 → P3) with foundational scaffolding first and release governance + verification last.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no incomplete-task dependencies).
- **[Story]**: User story label (US1, US2, US3) — required in story phases only.
- File paths are absolute or repo-rooted.

## Path Conventions

Single Rust project at repository root:

- `src/` — Rust source (binaries: `core-ops`, `core-ops-verify`, `core-ops-release`)
- `tests/integration/` — integration tests
- `tests/fixtures/` — test fixtures
- `specs/016-source-repository-layout/` — feature directory (specs, examples, skill bundle)
- `scripts/` — operator-facing scripts
- `changes/` — release-intent fragments

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create directory scaffolding and the release-intent fragment so every later task lands in a known place.

- [X] T001 Create directory scaffold: `specs/016-source-repository-layout/examples/`, `specs/016-source-repository-layout/skill/`, `scripts/` (if not present), `tests/fixtures/source_repos/`, `tests/fixtures/legacy_source_repo/`
- [X] T002 Create release-intent fragment at `changes/016-source-repository-layout.md` declaring `release_intent: major` with a one-paragraph summary of the layout change, the new `core-ops skill install` subcommand, and the legacy-parser removal

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Land the shared types, enums, errors, and state schema that every user story consumes.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T003 Add `LayoutError` enum to `src/core/errors.rs` (the only error in this module deriving `miette::Diagnostic` — deliberate asymmetry justified by parser source-span needs). Internal parser types (`SourceRepository`, `ParsedService`, `ParsedHost`, `PayloadKind`, `UnitName`, `UnitExtension`, `RelativePath`, etc.) live inside `src/io/repo.rs` per revised data-model.md and are added in Phase 3 as part of T107–T116. `ServiceDefinition`/`HostOverlaySet`/`EvaluationInput` in `src/core/types.rs` are preserved unchanged.
- [X] T004 Add `layout_version: Option<String>` field to `DesiredStateProvenance` in `src/core/types.rs` with `#[serde(rename = "layout-version", default, skip_serializing_if = "Option::is_none")]`. Schema version is NOT bumped (additive optional field with serde default). The originally-specified per-service `config_root` field is unnecessary because `ConfigFileSource.target_path` already encodes the resolved `/etc/<config-root>/...` path.
- [X] T005 Update `DesiredStateProvenance` construction sites: `src/io/state.rs` (`persist_never_run_state`, `build_state`) write `Some("1".to_string())`; pre-existing test fixtures (`tests/unit/test_rollback_detached.rs`, `tests/integration/test_plan.rs`, `tests/unit/test_agent_state.rs`, `tests/unit/test_state_snapshot.rs`) write `None` to preserve their pre-016 semantics. `cargo build --tests` and `cargo clippy --all-targets -- -D warnings` both green.

**Checkpoint**: Foundation ready — user-story work can begin.

---

## Phase 3: User Story 1 — Author a service from spec alone (Priority: P1) 🎯 MVP

**Goal**: A reader of `spec.md` and one example can author a conformant service or host overlay; the loader accepts it and produces a correct plan.

**Independent Test**: Given the spec and one of the in-tree examples, an outside reviewer authors a fresh service in a scratch repo. `core-ops plan` succeeds and the planned destination paths match expectations on first attempt without source-code reading.

### Tests for User Story 1 (REQUIRED) ⚠️

> Write these tests FIRST. Confirm they FAIL before implementation begins.

- [X] T101 [P] [US1] Author `specs/016-source-repository-layout/examples/01-minimal-single-service/` — one service (no `service.yaml`), one Quadlet `*.container`, one `config/` file; documented `README.md` describing the example [SUPERSEDED by spec/017]
- [X] T102 [P] [US1] Author `specs/016-source-repository-layout/examples/02-variant-config-root/` — one service whose id differs from `config-root` (modeled on `traefik-dnschallenge` → `/etc/traefik/`), with `service.yaml` and a `config/` file; `README.md` [SUPERSEDED by spec/017]
- [X] T103 [P] [US1] Author `specs/016-source-repository-layout/examples/03-multi-unit-with-dropins/` — service with both `quadlet/` (one `*.container` + drop-in) and `systemd/` (one `*.socket` + drop-in); `README.md` [SUPERSEDED by spec/017]
- [X] T104 [P] [US1] Author `specs/016-source-repository-layout/examples/04-host-overlay/` — base service with units, plus a host directly under `hosts/<id>/<svc>/` contributing a drop-in and a `config/` whole-file replacement; `README.md` [SUPERSEDED by spec/017]
- [X] T105 [P] [US1] Add `tests/integration/test_source_repo_layout.rs` with `#[test]` functions covering: each example loads cleanly (4 tests), reserved-name rejection (FR-009), config path traversal (FR-010), destination conflict (FR-011), legacy artifact rejection (FR-012), orphan drop-in (FR-013), determinism — repeated load yields identical `DesiredState` (FR-014/FR-015), missing-service diagnostic (FR-016), malformed `service.yaml` diagnostic for unknown key and parse error (FR-017), host overlay base-unit rejection (FR-018). Each test asserts diagnostic message content.
- [X] T106 [P] [US1] Authored `tests/fixtures/verification/scenarios/source-repo-variant-config-root.yaml` plus a single-revision repo fixture under `tests/fixtures/verification/repos/source-repo-variant-config-root/source-repo-variant-config-root-v1/` (services/traefik-dnschallenge with `service.yaml: { config-root: traefik }` + `config/traefik.yaml` + container quadlet, plus hosts/example-host/host.yaml). The scenario boots a guest, runs init+apply (with `host: example-host` so the parser pins to a known host id rather than the auto-generated VM hostname), then exercises four `guest_command` checks: `/etc/traefik/traefik.yaml` exists, contains the expected payload, `/etc/traefik-dnschallenge/` does NOT exist, and `/etc/containers/systemd/traefik-dnschallenge.container` exists. Synthetic-mode dry-run passes 7/7 assertions; running against a real VM is T406.

### Implementation for User Story 1

> All implementation tasks T107–T118 edit `src/io/repo.rs` (or its callers); they are sequential within the same file. Run after T101–T106 land and the tests fail as expected.

- [X] T107–T113, T116, T117 [US1] **Parser rewrite landed in `src/io/repo.rs`** (file went from 1107 → 1393 LOC). Single coherent rewrite implementing: legacy artifact rejection at every entry point (`validate_no_legacy_root_artifacts` + per-site `RepoError::LegacyArtifact`); reserved-name validation (`validate_id`, FR-009); `service.yaml` deserialization with `deny_unknown_fields` and kebab-case (`ServiceManifest`, FR-006/FR-007); payload-kind dispatch via `PayloadKind` enum + `read_payload_units` + `read_payload_dropins` (FR-003/FR-004); `read_config_files(config_dir, config_root)` with path-traversal rejection (FR-010); host overlay walk via `walk_host_service_overlay` rejecting base units (FR-018) and walking only `<unit>.<ext>.d/` drop-ins; deterministic order via existing BTreeMap traversal (FR-014/FR-015). `load_host_overrides` signature now takes `&ServiceCatalog` for config-root lookup. `HostYaml` deserializer hardened with `deny_unknown_fields` (FR-008). Planner consumers (`src/cli/plan.rs`, `src/io/apply.rs`, `src/core/validation.rs`) untouched — the parser preserves the existing `EvaluationInput { host, catalog, overlays }` contract. `cargo build --tests` and `cargo clippy --all-targets -- -D warnings` ✅
- [X] T114 [US1] Verify orphan drop-in detection (FR-013) — confirmed transitively covered by existing `core::validation::validate_dropin_targets` which the parser still calls. T105's `orphan_dropin_rejected` exercises the path; no parser change required.
- [X] T115 [US1] Verify destination conflict detection (FR-011) — gap surfaced (the existing `validate_workloads`/`DuplicateUnitName` only catches quadlet collisions, not `config_files` target_path collisions). Added `validate_config_destination_conflicts` in `src/io/repo.rs`, called from both `load_layered_repo` and `load_layered_desired_state`; surfaces `RepoError::DestinationConflict { target, a, b }`. (commit `6364dc6`)
- [X] T118 [US1] Removed `tests/fixtures/layered_overrides/`. Of the 6 dependents the handoff named, only 3 actually used the fixture: `test_host_overrides.rs` (rewritten with inline fixture for drop-in ordering + socket precedence), `test_overlay_validation.rs` (deleted; subsumed by T105's `orphan_dropin_rejected`), `test_service_selection.rs` (one test rewritten with inline multi-host fixture; the second was subsumed by T105's `example_02_variant_config_root_loads`). The other 3 (`test_quickstart_validation`, `tests/unit/test_dropin_order`, `tests/unit/test_evaluation_determinism`) did not actually reference the fixture and need no edits per the data-model scope correction (`HostOverlaySet` is preserved). Helpers extracted to `tests/integration/source_repo_support.rs`.
- [X] T119 [US1] `cargo test` and `cargo clippy --all-targets -- -D warnings` both green. Required broader cleanup than the handoff scoped: 15 additional integration tests across 9 files were still authoring legacy `quadlets/` fixtures inline and had to be migrated to the formalized layout. Also fixed two cross-cutting hazards that surfaced once the suite became runnable: every `path_lock()` site now uses `.unwrap_or_else(|err| err.into_inner())` so one panic no longer poison-cascades the rest, and the three `cli::explain` unit tests racing on `STATE_FILE_ENV` are now serialized via a module-local `OnceLock<Mutex>`. Final result: 25 lib unit + 409 integration tests pass, clippy clean.

**Checkpoint**: User Story 1 fully functional. Layout formalization is shipped — operators can author services from the spec alone. **Stop here for MVP demo.**

---

## Phase 4: User Story 2 — Install the agent skill (Priority: P2)

**Goal**: An operator runs `core-ops skill install` and an agent reading the resulting `SKILL.md` can author conformant services without further coaching.

**Independent Test**: After `core-ops skill install` in a scratch directory, `.agents/skills/core-ops-source-repo/SKILL.md` exists with the bundled content. An external agent given only that file authors a service the loader accepts on first attempt.

### Tests for User Story 2 (REQUIRED) ⚠️

- [X] T201 [P] [US2] Authored `specs/016-source-repository-layout/skill/SKILL.md` (12 sections: layout, identifier rules, service.yaml, host.yaml, dispatch table, host overlay semantics, drop-in conventions, validation rules, determinism, legacy artifacts, four worked walk-throughs, authoring checklist). Self-contained — every example tree is inlined; nothing reaches outside the skill at runtime.
- [X] T202 [P] [US2] Added `tests/integration/test_skill_install.rs` with all 6 contract test functions. Tests run the binary as a subprocess via `CARGO_BIN_EXE_core-ops`; `--global` runs hold `path_lock()` and override `HOME` to a tempdir via a `HomeGuard` helper. Confirmed FAIL pre-implementation (clap rejects unknown `skill` subcommand).

### Implementation for User Story 2

- [X] T203 [US2] `Skill(SkillArgs)` variant added to `Commands` in `src/cli/args.rs`. `SkillArgs { op: SkillOp }` (subcommand wrapper), `SkillOp::Install(SkillInstallArgs)`, `SkillInstallArgs { global: bool, print: bool }` with `#[arg(long, conflicts_with = ...)]` on each flag.
- [X] T204 [US2] `src/cli/skill.rs` implements the install subcommand. `SKILL_BUNDLE: &[(&str, &[u8])]` embeds `SKILL.md` via `include_bytes!`. `run(args) -> miette::Result<()>` resolves destination (cwd, `$HOME`, or stdout per the flag), creates parent directories, and writes each entry byte-identically. Idempotent: pre-existing byte-identical files are skipped (mtime preserved); pre-existing files with diverged bytes raise a `miette` diagnostic naming the path. `print_bundle` writes `==> <relative-path> <==\n<bytes>` per the contract's bundle stream format. Three module-local unit tests (round-trip, idempotent, divergent-content rejection) added; counted under the lib unit-test result.
- [X] T205 [US2] `pub mod skill;` wired in `src/cli/mod.rs`. `main.rs` dispatches `Commands::Skill(args) => match args.op { SkillOp::Install(install_args) => ... }`. Errors render as `miette::Report` debug output and `std::process::exit(1)`, satisfying the contract's "non-zero on error / surfaced as miette diagnostics" requirement without funnelling through the existing `CoreError` pipeline.
- [X] T206 [US2] `cargo test`: 28 lib unit + 415 integration = 443 pass. `cargo clippy --all-targets -- -D warnings` ✅.

**Checkpoint**: Both US1 and US2 are functional. The skill bundle is installable; agents have authoring guidance.

---

## Phase 5: User Story 3 — Migrate the live legacy repository (Priority: P3)

**Goal**: The legacy `~/code/ulthar/repo/` (and any structurally-identical source repository) can be migrated to the formalized layout in one mechanical pass; post-migration plan output's destination set matches pre-migration exactly.

**Independent Test**: Given a copy of the legacy fixture, run `scripts/migrate-legacy-source-repo.sh <path>`; the new parser loads the result without error; `core-ops plan` produces a destination set identical to the pre-migration plan.

### Tests for User Story 3 (REQUIRED) ⚠️

- [X] T301 [P] [US3] Authored `tests/fixtures/legacy_source_repo/` — minimal legacy-shape fixture exercising every D10 transformation: kind reassignment (`quadlet/traefik.socket` → `systemd/`), split-overrides flattening (`quadlet-overrides/` → `quadlet/`), config etc-mirror flattening (`config/etc/<root>/` → `config/`), variant config-root requiring synthesized `service.yaml`, and host overrides for both drop-ins and config files. `expected-destinations.txt` records the post-migration plan destination set (5 lines, sorted) the test asserts against.
- [X] T302 [P] [US3] Added `tests/integration/test_migrate_legacy.rs` with four tests: (a) migrated tree loads via `load_with_host`; (b) destination set matches `expected-destinations.txt` byte-for-byte; (c) migration is idempotent (FNV-1a snapshot of the tree before/after the second run is identical); (d) variant `traefik-dnschallenge` emerges with a `service.yaml` declaring `config-root: traefik`. Confirmed FAIL pre-implementation (`No such file or directory` for the missing script).

### Implementation for User Story 3

- [X] T303 [US3] `scripts/migrate-legacy-source-repo.sh` implements the D10 migration in two phases: per-service (kind reassignment, split-override flattening, config etc-mirror flattening with `service.yaml` synthesis for variants) followed by per-host (host drop-in re-rooting via the `unit_owner` lookup, host config override re-rooting via the `config-root → service` lookup). File-moves only via `mv`. Idempotent: every legacy directory is removed via `rmdir` / `find -empty -delete` after migration so a second run finds nothing to do. Multi-owner host drop-ins fail loudly via `unit_owner`'s exit-2 path. Single-argument CLI; exits 64 on usage error, 65 on layout error, 66 on path error.
- [X] T304 [US3] `cargo test` green: 28 lib unit + 419 integration = 447 pass (4 of which are the new migration tests). `cargo clippy --all-targets -- -D warnings` ✅. `shellcheck` is not installed in this dev environment; `e2e-gate.yml` in CI is the binding shellcheck gate (per the existing release-validation flow).

**Checkpoint**: All three user stories are functional. The live legacy repository can be migrated whenever the operator chooses.

---

## Phase 6: Polish & Release Governance

**Purpose**: Bring the change set to release-ready state per constitution principle 13 and run the full validation gate.

- [X] T401 `Cargo.toml` version bumped 1.0.0 → 2.0.0. `tests/fixtures/provenance_state/valid-success.json` updated to track (the contract test `controller_version_provenance_matches_cargo_package_version` pins this).
- [X] T402 `cargo run --bin core-ops-release -- changelog` rendering verified. The fragment summary was rewritten to enumerate all five user-visible deliverables (formalized layout, legacy-parser removal, `core-ops skill install`, four in-tree examples, migration script) so the rendered `[Unreleased]` bullet captures them. The machine-managed block in `CHANGELOG.md` was synced to match the rendering.
- [X] T403 `cargo run --bin core-ops-release -- validate --base-ref HEAD^` → **passed**. Required bump `minor`, declared `major` (allowed; explicit major because the public CLI surface lost the legacy-parser code paths and gained the `skill` subcommand).
- [X] T404 [P] `cargo test` green: 28 lib unit + 419 integration = 447 pass. `cargo clippy --all-targets -- -D warnings` ✅.
- [X] T405 [P] Quickstart walkthrough verified. Steps 1–5 (authoring) are exercised by the four in-tree examples + the migration test; Step 6 surfaces (`core-ops plan`, `core-ops apply`) are covered by the integration suite (`test_plan`, `test_apply_report`, `test_reconcile_apply`); Step 7 (`core-ops skill install` with `--global` and `--print`) by `test_skill_install`. CLI help confirms the `skill install` flags are reachable: `--global`, `--print` (mutually exclusive per clap `conflicts_with`).
- [X] T406 **(release-blocker)** VM-backed scenario `source-repo-variant-config-root` ran against the operator's libvirt host (`CORE_OPS_VERIFY_VM_HOST=ulthar`, qemu+ssh) and **passed all 7 assertions in ~60s**: init-succeeded, apply-succeeded, apply-converged, variant-config-file-deployed (`/etc/traefik/traefik.yaml` exists), variant-config-content-matches (`level: INFO` in the file), svcid-config-dir-not-created (`/etc/traefik-dnschallenge/` absent), container-quadlet-deployed (`/etc/containers/systemd/traefik-dnschallenge.container` exists). The apply log on the live VM showed `2 creates / Outcome: converged`. Run ID: `run-1777662397344944784-accepted-corpus`. Required adding `libvirt` to `flake.nix`'s devshell so `virsh` resolves on PATH. FR-006/FR-007 (variant config-root deploys to `/etc/<config-root>/`, not `/etc/<svc-id>/`) is validated end-to-end on real systemd. Constitution principle 10 satisfied; release tag is unblocked.
- [X] T407 `AGENTS.md` "Recent Changes" gained a leading entry for 016 covering the layout formalization, the `core-ops skill install` subcommand, the in-tree examples + migration script, and the major version bump.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)** → no external dependencies; can start immediately.
- **Foundational (Phase 2)** → depends on Setup; **blocks all user-story work**.
- **US1 (Phase 3)** → depends on Foundational; the MVP target.
- **US2 (Phase 4)** → depends on Foundational; technically independent of US1 because the skill subcommand and the layout parser touch disjoint files (`src/cli/skill.rs`, `src/cli/args.rs`, embedded `SKILL.md`). However, US2 SHOULD NOT ship before US1: the bundled `SKILL.md` teaches the formalized layout, and shipping it before US1 implements that layout means the binary can't yet load what the skill teaches. Develop in parallel if you want; release together.
- **US3 (Phase 5)** → depends on Foundational; ideally also depends on US1 because the migration script is best validated by loading the result with the post-rewrite parser (T302's assertion (a)).
- **Polish (Phase 6)** → depends on every desired user story being green.

### Within Each User Story

- Tests authored first (T101–T106, T201–T202, T301–T302). Confirm they FAIL before implementation begins.
- Implementation tasks within a story are mostly sequential because they edit shared files (`src/io/repo.rs` for US1, `src/cli/skill.rs` plus `args.rs` for US2). Splits between distinct files are flagged `[P]`.
- Each story closes with `cargo test` and `cargo clippy --all-targets -- -D warnings` green (T119, T206, T304).

### Parallel Opportunities

- **Phase 1**: T001 and T002 are sequential but trivial.
- **Phase 2**: T003, T004, T005 are sequential because T003 and T004 share `src/core/types.rs`; T005 reads the new types.
- **Phase 3 tests**: T101, T102, T103, T104 are independent example directories — fully `[P]`. T105 (the integration test file) is `[P]` with the example authoring (different file). T106 (the VM scenario) is `[P]` with all of T101–T105.
- **Phase 3 impl**: T107–T117 are sequential within `src/io/repo.rs`. T117 touches `plan.rs` and `apply.rs` (different files than T107–T116) but logically depends on the new types being landed first. T118 (fixture cleanup) can run any time after T117 lands.
- **Phase 4 tests**: T201 (SKILL.md authoring) and T202 (test file) are `[P]`.
- **Phase 4 impl**: T203 → T204 → T205 are sequential.
- **Phase 5**: T301 (fixture) and T302 (test file) are `[P]`. T303 (script) follows.
- **Phase 6**: T403 must run after T401 and T402 (version + changelog). T404, T405, T406 are `[P]` after T401–T403.

---

## Parallel Example: User Story 1

```bash
# After Phase 2 lands, kick off these in parallel (different files, no incomplete-task deps):
Task: "T101 — author examples/01-minimal-single-service/"
Task: "T102 — author examples/02-variant-config-root/"
Task: "T103 — author examples/03-multi-unit-with-dropins/"
Task: "T104 — author examples/04-host-overlay/"
Task: "T105 — author tests/integration/test_source_repo_layout.rs"
Task: "T106 — author tests/fixtures/verification/scenarios/source-repo-variant-config-root.yaml"

# Then sequentially in src/io/repo.rs:
T107 → T108 → T109 → T110 → T111 → T112 → T113 → T114 → T115 → T116

# Then in parallel (different files):
Task: "T117 — update src/cli/plan.rs"
Task: "T117 — update src/io/apply.rs"
Task: "T118 — remove tests/fixtures/layered_overrides/, update referencing tests"

# Finally:
T119 — cargo test && cargo clippy
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Phase 1 (Setup) — minutes.
2. Phase 2 (Foundational) — types + state schema.
3. Phase 3 (US1) — write tests, confirm they fail, implement, confirm they pass.
4. **STOP and VALIDATE**: live ulthar/repo cannot yet be loaded (legacy rejected — that's correct), but a fresh authored repo loads cleanly.
5. Demo against `examples/02-variant-config-root/` to confirm the variant case works.

### Incremental Delivery

1. MVP (US1) ships: layout formalized, legacy rejected with clear migration pointer.
2. Add US2 (skill install): operators get authoring guidance.
3. Add US3 (migration): the live ulthar/repo can be migrated and operations resume.
4. Polish: version bump, changelog, full validation gate.

### Parallel Team Strategy

Single-developer (likely scenario): MVP first, then US2, then US3, then Polish. ~30 tasks, mostly sequential within stories, ~9 days at one task per session.

Two developers: after Phase 2, dev A drives US1 (the bulk), dev B drives US2 in parallel (small, contained); merge before US3.

---

## Notes

- `[P]` = different files, no incomplete-task dependencies. Same-file edits are sequential.
- Tests are mandatory; the constitution does not grant exemption for parser changes.
- Provenance: status snapshot gains `layout-version: "1"` (T005) — verifiable via `core-ops status` output.
- Release governance: T002 (release-intent fragment), T401 (Cargo.toml), T402 (CHANGELOG), T403 (validate) are the four required touchpoints for principle 13.
- Verify each story's tests fail before implementing, and each story's gates are green before moving on.
- Commit after each task or logical group; the `after_implement` hook (auto-commit) is registered and optional.
- Stop at any checkpoint to validate independence: US1 alone is shippable; US2 and US3 add value without breaking US1.
- Avoid: vague tasks, cross-story dependencies that defeat independent-testability, accidental edits to legacy code paths during impl tasks.
