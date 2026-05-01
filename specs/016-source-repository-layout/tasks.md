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

- [X] T101 [P] [US1] Author `specs/016-source-repository-layout/examples/01-minimal-single-service/` — one service (no `service.yaml`), one Quadlet `*.container`, one `config/` file; documented `README.md` describing the example
- [X] T102 [P] [US1] Author `specs/016-source-repository-layout/examples/02-variant-config-root/` — one service whose id differs from `config-root` (modeled on `traefik-dnschallenge` → `/etc/traefik/`), with `service.yaml` and a `config/` file; `README.md`
- [X] T103 [P] [US1] Author `specs/016-source-repository-layout/examples/03-multi-unit-with-dropins/` — service with both `quadlet/` (one `*.container` + drop-in) and `systemd/` (one `*.socket` + drop-in); `README.md`
- [ ] T104 [P] [US1] Author `specs/016-source-repository-layout/examples/04-host-overlay/` — base service with units, plus a host directly under `hosts/<id>/<svc>/` contributing a drop-in and a `config/` whole-file replacement; `README.md`
- [ ] T105 [P] [US1] Add `tests/integration/test_source_repo_layout.rs` with `#[test]` functions covering: each example loads cleanly (4 tests), reserved-name rejection (FR-009), config path traversal (FR-010), destination conflict (FR-011), legacy artifact rejection (FR-012), orphan drop-in (FR-013), determinism — repeated load yields identical `DesiredState` (FR-014/FR-015), missing-service diagnostic (FR-016), malformed `service.yaml` diagnostic for unknown key and parse error (FR-017), host overlay base-unit rejection (FR-018). Each test asserts diagnostic message content.
- [ ] T106 [P] [US1] Add VM-backed scenario at `tests/fixtures/verification/scenarios/source-repo-variant-config-root.yaml` exercising a host that selects a service whose `config-root` differs from its id; assert deployed `/etc/<config-root>/` paths on the live host. Per constitution principle 10, this is mandatory for the externally visible config-root behavior.

### Implementation for User Story 1

> All implementation tasks T107–T118 edit `src/io/repo.rs` (or its callers); they are sequential within the same file. Run after T101–T106 land and the tests fail as expected.

- [X] T107–T113, T116, T117 [US1] **Parser rewrite landed in `src/io/repo.rs`** (file went from 1107 → 1393 LOC). Single coherent rewrite implementing: legacy artifact rejection at every entry point (`validate_no_legacy_root_artifacts` + per-site `RepoError::LegacyArtifact`); reserved-name validation (`validate_id`, FR-009); `service.yaml` deserialization with `deny_unknown_fields` and kebab-case (`ServiceManifest`, FR-006/FR-007); payload-kind dispatch via `PayloadKind` enum + `read_payload_units` + `read_payload_dropins` (FR-003/FR-004); `read_config_files(config_dir, config_root)` with path-traversal rejection (FR-010); host overlay walk via `walk_host_service_overlay` rejecting base units (FR-018) and walking only `<unit>.<ext>.d/` drop-ins; deterministic order via existing BTreeMap traversal (FR-014/FR-015). `load_host_overrides` signature now takes `&ServiceCatalog` for config-root lookup. `HostYaml` deserializer hardened with `deny_unknown_fields` (FR-008). Planner consumers (`src/cli/plan.rs`, `src/io/apply.rs`, `src/core/validation.rs`) untouched — the parser preserves the existing `EvaluationInput { host, catalog, overlays }` contract. `cargo build --tests` and `cargo clippy --all-targets -- -D warnings` ✅
- [ ] T114 [US1] Verify orphan drop-in detection (FR-013) — likely covered transitively by existing `core::validation::validate_dropin_targets` which the parser still calls. Add explicit test in T105 to confirm.
- [ ] T115 [US1] Verify destination conflict detection (FR-011) — partially covered by existing `core::validation::DuplicateWorkload` / `DuplicateUnitName` checks. Add explicit test in T105 to confirm; if gaps surface, add explicit conflict map in `selected_service_artifacts`.
- [ ] T118 [US1] Remove obsolete legacy fixtures and update referencing tests. Delete `tests/fixtures/layered_overrides/` and adapt every test that mentions it: `tests/integration/test_host_overrides.rs`, `tests/integration/test_overlay_validation.rs`, `tests/integration/test_quickstart_validation.rs`, `tests/integration/test_service_selection.rs`, `tests/unit/test_dropin_order.rs`, `tests/unit/test_evaluation_determinism.rs`. Source files that reference the legacy `HostOverlaySet` type — `src/core/types.rs` and `src/io/repo.rs` — are already rewritten by T003/T107 and need no further attention here. Replace fixture references with paths into the new `specs/016-source-repository-layout/examples/` tree where appropriate; delete tests whose semantics are now subsumed by `test_source_repo_layout.rs` (T105). Run `grep -r layered_overrides\\|HostOverlaySet src tests` and confirm zero matches before marking the task complete.
- [ ] T119 [US1] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; fix every failure or warning. US1 is not complete until both gates are green.

**Checkpoint**: User Story 1 fully functional. Layout formalization is shipped — operators can author services from the spec alone. **Stop here for MVP demo.**

---

## Phase 4: User Story 2 — Install the agent skill (Priority: P2)

**Goal**: An operator runs `core-ops skill install` and an agent reading the resulting `SKILL.md` can author conformant services without further coaching.

**Independent Test**: After `core-ops skill install` in a scratch directory, `.agents/skills/core-ops-source-repo/SKILL.md` exists with the bundled content. An external agent given only that file authors a service the loader accepts on first attempt.

### Tests for User Story 2 (REQUIRED) ⚠️

- [ ] T201 [P] [US2] Author `specs/016-source-repository-layout/skill/SKILL.md`: covers canonical layout, `service.yaml` schema, payload-kind dispatch table, host-overlay semantics, drop-in conventions, validation rules (FR-009 to FR-013), and a worked authoring walk-through producing each of the four example shapes (minimal, variant, multi-unit, host-overlay). The skill MUST be self-contained: reference examples by short identifier (`01-minimal-single-service`, `02-variant-config-root`, `03-multi-unit-with-dropins`, `04-host-overlay`) and inline the relevant directory tree for each example. Note in the skill that the originals live at `specs/016-source-repository-layout/examples/` in the `core-ops` repository, but do NOT use relative paths that assume the skill is co-located with the spec — the installed location is `.agents/skills/core-ops-source-repo/SKILL.md`, isolated from the source repo it documents.
- [ ] T202 [P] [US2] Add `tests/integration/test_skill_install.rs` with the 6 named test functions from `specs/016-source-repository-layout/contracts/skill-cli.md`: `test_skill_install_default`, `test_skill_install_global` (with `tempdir`-overridden `$HOME`), `test_skill_install_print`, `test_skill_install_idempotent`, `test_skill_install_no_init_coupling`, `test_skill_install_vendor_neutral` (assert destination contains `.agents/skills/` and never `.claude/skills/`)

### Implementation for User Story 2

- [ ] T203 [US2] Add `Skill(SkillArgs)` variant to the top-level `Command` enum in `src/cli/args.rs`; define `SkillArgs`, `SkillOp::Install(SkillInstallArgs)`, `SkillInstallArgs { global: bool, print: bool }` with `clap` `conflicts_with` on the two flags
- [ ] T204 [US2] Create `src/cli/skill.rs` implementing the install subcommand. Bundle entries are embedded at compile time: `const SKILL_BUNDLE: &[(&str, &[u8])] = &[("SKILL.md", include_bytes!("../../specs/016-source-repository-layout/skill/SKILL.md"))];` (path is relative to `src/cli/skill.rs`). `run(args) -> miette::Result<()>` resolves destination — `<cwd>/.agents/skills/core-ops-source-repo/` by default, `$HOME/.agents/skills/core-ops-source-repo/` under `--global`, standard output under `--print` — then writes (or prints) every entry byte-identically per the bundle stream format in `contracts/skill-cli.md`. Refuses to overwrite existing files whose bytes differ from the bundle, with a `miette` diagnostic naming the offending path.
- [ ] T205 [US2] Wire `pub mod skill;` in `src/cli/mod.rs` and dispatch the new `Command::Skill` arm in the main CLI runtime (likely `src/main.rs` or `src/cli/mod.rs::run`)
- [ ] T206 [US2] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; fix every failure or warning.

**Checkpoint**: Both US1 and US2 are functional. The skill bundle is installable; agents have authoring guidance.

---

## Phase 5: User Story 3 — Migrate the live legacy repository (Priority: P3)

**Goal**: The legacy `~/code/ulthar/repo/` (and any structurally-identical source repository) can be migrated to the formalized layout in one mechanical pass; post-migration plan output's destination set matches pre-migration exactly.

**Independent Test**: Given a copy of the legacy fixture, run `scripts/migrate-legacy-source-repo.sh <path>`; the new parser loads the result without error; `core-ops plan` produces a destination set identical to the pre-migration plan.

### Tests for User Story 3 (REQUIRED) ⚠️

- [ ] T301 [P] [US3] Author `tests/fixtures/legacy_source_repo/` — a minimal legacy-shaped fixture covering: `services/<svc>/quadlet/`, `services/<svc>/quadlet-overrides/<unit>.<ext>.d/<file>.conf`, `services/<svc>/config/etc/<svc>/<file>`, `services/<other>/config/etc/<different>/<file>` (variant requiring `service.yaml`), `hosts/<h>/overrides/quadlet/<unit>.<ext>.d/<file>.conf`, `hosts/<h>/overrides/config/etc/<svc>/<file>`. This fixture exists for migration testing only.
- [ ] T302 [P] [US3] Add `tests/integration/test_migrate_legacy.rs`: copy `tests/fixtures/legacy_source_repo/` to a `tempfile::TempDir`, run the migration script via `std::process::Command`, assert: (a) the new parser loads the migrated tree without error, (b) `core-ops plan` against the migrated tree produces the same destination set as a pre-migration plan snapshot recorded in the fixture as `expected-destinations.txt`, (c) re-running the migration script on an already-migrated tree is idempotent (no-op exit), (d) variant services emerge with a `service.yaml` declaring the correct `config-root`

### Implementation for User Story 3

- [ ] T303 [US3] Implement `scripts/migrate-legacy-source-repo.sh` per the migration table in `specs/016-source-repository-layout/research.md` D10. File moves only; no semantic re-interpretation. Idempotent (re-running is a no-op). Single-argument `<path-to-source-repo>`. Exits non-zero with a clear error if the path is not a recognized layout (legacy or already-formalized). Before migrating any host drop-in, the script MUST build a **unit→service ownership map** by walking every `services/<svc>/quadlet/<unit>.<ext>` and `services/<svc>/systemd/<unit>.<ext>` and recording the owning service id; this map is used to determine the destination subdirectory for every `hosts/<h>/overrides/quadlet/<unit>.<ext>.d/...` entry. If a host drop-in references a unit owned by zero or more than one service, the script MUST fail loudly naming the offending unit and explaining the manual resolution path (rename the unit, or split the drop-in by service). Pass `shellcheck -S warning scripts/migrate-legacy-source-repo.sh`.
- [ ] T304 [US3] Run `cargo test` (US3 tests) and `shellcheck -S warning scripts/migrate-legacy-source-repo.sh`; fix every failure.

**Checkpoint**: All three user stories are functional. The live legacy repository can be migrated whenever the operator chooses.

---

## Phase 6: Polish & Release Governance

**Purpose**: Bring the change set to release-ready state per constitution principle 13 and run the full validation gate.

- [ ] T401 Bump `Cargo.toml` `version` from `1.0.0` to `2.0.0` (the canonical controller version, per constitution principle 12)
- [ ] T402 Verify `CHANGELOG.md` `[Unreleased]` rendering. The block between `<!-- core-ops-release:start -->` and `<!-- core-ops-release:end -->` is **machine-managed**; do NOT hand-edit it. Per CLAUDE.md: run `cargo run --bin core-ops-release -- changelog > /tmp/new-changelog.md` and visually confirm the generated `[Unreleased]` block enumerates the formalized layout, the legacy-parser removal, the new `core-ops skill install` subcommand, the in-tree examples, and the migration script — all derived from the release-intent fragment authored in T002. If anything is missing, fix the fragment (T002), not the changelog. CI rewrites the changelog on tag.
- [ ] T403 Run `cargo run --bin core-ops-release -- validate --base-ref HEAD^` and confirm green; the command verifies the release-intent fragment, version bump, and changelog alignment
- [ ] T404 [P] Final full validation: `cargo test` and `cargo clippy --all-targets -- -D warnings` across the entire workspace, all binaries, all tests
- [ ] T405 [P] Walk through `specs/016-source-repository-layout/quickstart.md` end-to-end manually (or via a scripted check): create the skeleton, author each service, run plan, run `skill install`, run migration; confirm every step works as documented
- [ ] T406 **(release-blocker)** Run the VM-backed scenario from T106 against a libvirt-managed VM per the existing verification harness and record the result. Per constitution principle 10, the feature is NOT release-complete until this scenario passes (or an explicit exemption with justification is recorded in this task). T406 is in the Polish phase for ordering convenience but is not optional — a failure here blocks the release regardless of whether T401–T405 are green. If the scenario reveals gaps, update `specs/016-source-repository-layout/spec.md` Verification Guidance and re-run.
- [ ] T407 Update `AGENTS.md` "Recent Changes" entry for 016 if the auto-generated entry from `update-agent-context.sh` is awkwardly worded; preserve the format of prior entries

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
