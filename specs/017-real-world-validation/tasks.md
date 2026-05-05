# Tasks: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Input**: Design documents from `/home/outergod/code/github.com/outergod/core-ops/specs/017-real-world-validation/`
**Prerequisites**: plan.md (✅), spec.md (✅), research.md (✅), data-model.md (✅), contracts/ (✅), quickstart.md (✅)

**Tests**: REQUIRED. FR-006 explicitly mandates per-example integration tests; FR-016 mandates help-text contracts; spec.md Constitution Alignment requires `cargo test` and `cargo clippy --all-targets -- -D warnings` to pass before merge. The VM-backed-scenario exemption is recorded explicitly in spec.md and plan.md.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing. The validation iteration's user stories are:

- **US1 (P1)** — First-time operator runs a real example with one command (stateless plan + explain + 5 examples)
- **US2 (P2)** — Operator authors and iterates on their own setup without committing first (non-git stateless support, scaffolding ergonomics)
- **US3 (P2)** — Stateless apply for one-off convergence and recovery (apply provenance, init'd state preservation)
- **US4 (P3)** — Future spec author grounds amendments in validation evidence (synthesis table)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4); omitted for Setup/Foundational/Polish phases
- File paths are absolute or repo-root-relative

## Path Conventions

Single Rust project per `plan.md`. Repo-root-relative paths for `src/`, `tests/`, `examples/`, `docs/`, `specs/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Clear the way for spec/017's deliverables. Removes the four superseded spec/016 example fixtures and their single in-tree consumer so the new top-level `examples/` doesn't fight the old.

- [X] T001 Remove the four spec/016 example fixture directories: `git rm -r specs/016-source-repository-layout/examples/01-minimal-single-service specs/016-source-repository-layout/examples/02-variant-config-root specs/016-source-repository-layout/examples/03-multi-unit-with-dropins specs/016-source-repository-layout/examples/04-host-overlay`
- [X] T002 Update `tests/integration/source_repo_support.rs:20`: repoint `EXAMPLES_DIR` const at top-level `examples` (or delete the const + helper if no surviving consumer remains after T001 + T026–T030 land); audit any `examples_root()` callers and fix or remove. Run `cargo check --tests` to confirm no dangling references.
- [X] T003 [P] Annotate `specs/016-source-repository-layout/spec.md` FR-023: append a supersession note pointing at top-level `examples/` and at this spec (`specs/017-real-world-validation/`). Do not remove the FR text — preserve historical record.
- [X] T004 [P] Annotate `specs/016-source-repository-layout/tasks.md` T101–T104: append ` [SUPERSEDED by spec/017]` to each line (the lines are already marked `[X]`; do not duplicate the marker). Preserve historical record.
- [X] T005 [P] Verify `scripts/migrate-legacy-source-repo.sh` does not reference the spec/016 example paths (per research.md D10). If it does, capture the references for a follow-up task; if not, confirm in the commit message.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Land the `--source-repo` CLI surface and the git-aware provenance helper that every user story depends on. No user-story phase can begin until this phase is complete and `cargo test` + `cargo clippy --all-targets -- -D warnings` are green.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 Add `source_repo: Option<PathBuf>` field with `requires("host")` and `ArgGroup` semantics to `PlanArgs` in `src/cli/args.rs`. Include doc-comments naming the flag in `--help`, the `--host` requirement, and the bypass relationship with `core-ops init` (per FR-016 / contracts/cli-flag.md help-text contract).
- [X] T007 Add the same `source_repo: Option<PathBuf>` field with `requires("host")` to `ApplyArgs` in `src/cli/args.rs` (same file as T006 → sequential).
- [X] T008 Add the same `source_repo: Option<PathBuf>` field with `requires("host")` to `ExplainArgs` in `src/cli/args.rs` (same file → sequential after T007).
- [X] T009 [P] Implement `src/io/source_ref.rs::detect_provenance(path: &Path) -> Result<DesiredStateProvenance, LayoutError>` per research.md D1 algorithm: canonicalize path; check `git -C <path> rev-parse --is-inside-work-tree`; check `git -C <path> status --porcelain -- .` for cleanliness; capture `git -C <path> rev-parse HEAD` SHA; emit `(stateless)` / `(stateless+dirty)` sentinels for non-git or dirty cases. Use `std::process::Command::new("git")` per the existing pattern at `src/cli/init.rs:52`. Validate path-is-directory with miette diagnostics emitting exit codes 64 / 65 / 66 per contracts/cli-flag.md.
- [X] T010 [P] Wire stateless source resolution into `src/cli/plan.rs`: branch on `args.source_repo`; when present, use `detect_provenance(path)` to build `EvaluationInput` directly; bypass init'd-state lookup; preserve existing `--audit-dir` handling unchanged (per FR-012 + 2026-05-05 clarification Q4); ensure no writes under `/var/lib/core-ops/`.
- [X] T011 [P] Wire stateless source resolution into `src/cli/apply.rs`: same as T010, plus apply-specific provenance recording — `desired_state.repository = canonicalize(path)`, `desired_state.requested_ref` = SHA-or-sentinel from `detect_provenance`. CRITICAL: do NOT mutate any existing init'd `desired_state.*` fields when stateless flag is present (FR-013, SC-009).
- [X] T012 [P] Wire stateless source resolution into `src/cli/explain.rs`: pure-read path; build `EvaluationInput` from `--source-repo`; ensure no writes anywhere; reuse existing explain rendering.
- [X] T013 Unit tests for clap argument parsing in `src/cli/args.rs::tests` (or a new `src/cli/args_tests.rs`): `--source-repo` accepted on plan/apply/explain with `--host`; `--source-repo` rejected on init/agent/status/skill; `--source-repo` without `--host` errors with the expected clap-generated message.
- [X] T014 [P] Unit tests for `src/io/source_ref.rs`: temp-dir non-git → `(stateless)`; temp-dir git-init clean → 40-char SHA; temp-dir git-init with uncommitted file → `(stateless+dirty)`; non-existent path → exit-64 error; path-is-file → exit-65 error.
- [X] T015 Run `cargo build`, `cargo test`, and `cargo clippy --all-targets -- -D warnings`. ALL MUST PASS before Phase 3 begins. This is the foundational checkpoint.

**Checkpoint**: Foundation ready — user stories US1, US2, US3 can now begin in parallel.

---

## Phase 3: User Story 1 — First-time operator runs a real example with one command (Priority: P1) 🎯 MVP

**Goal**: Five real-world examples published under `examples/<NN-slug>/`, each runnable via `core-ops plan --source-repo examples/<NN-slug> --host <host>` exit 0 with a non-empty plan, no `core-ops init` required.

**Independent Test**: A reviewer who has never seen CoreOps clones the repo, runs the suggested invocation against any of the five examples, and gets a populated plan. SC-001, SC-003, SC-006, SC-007, SC-008.

### Tests for User Story 1 (REQUIRED) ⚠️

> Write tests FIRST when feasible; for example-authoring tasks the example dirs need to exist before the parse test can fail-then-pass — author the example and the test as a paired commit per task.

- [X] T016 [P] [US1] Per-example integration test `tests/integration/test_examples_01_caddy_whoami.rs`: load `examples/01-caddy-whoami/` via the parser, assert (a) `Repository::load` succeeds, (b) resolved service catalog contains `caddy`, (c) example root carries `README.md`, (d) `cargo run --bin core-ops -- plan --source-repo examples/01-caddy-whoami --host example` exits 0 (via `assert_cmd`).
- [X] T017 [P] [US1] Per-example integration test `tests/integration/test_examples_02_nextcloud.rs`: same pattern; assert resolved services contain at minimum `nextcloud`, `nextcloud-db`, `nextcloud-redis`, `traefik` (or whichever ids the implementer locks during T022).
- [X] T018 [P] [US1] Per-example integration test `tests/integration/test_examples_03_immich.rs`: same pattern; assert services contain `immich-server`, `immich-database`, `immich-redis`, `immich-ml`, `traefik`.
- [X] T019 [P] [US1] Per-example integration test `tests/integration/test_examples_04_traefik_authelia.rs`: same pattern; assert services contain `traefik`, `authelia`, and at least one protected backend.
- [X] T020 [P] [US1] Per-example integration test `tests/integration/test_examples_05_observability.rs`: same pattern; assert services contain `prometheus`, `grafana`, `node-exporter`, `cadvisor`.
- [X] T021 [P] [US1] Stateless plan integration test `tests/integration/test_stateless_plan.rs`: cover (a) `--source-repo` against a non-git tempdir → exit 0 with `(stateless)` provenance, (b) clean git checkout → SHA provenance, (c) dirty working tree → `(stateless+dirty)` provenance, (d) missing `--host` → clap exit 2, (e) non-directory path → exit 64, (f) `--audit-dir` honored when explicitly set.
- [X] T022 [P] [US1] Stateless explain integration test `tests/integration/test_stateless_explain.rs`: pure-read invocation against **each of the five examples** (one sub-test per example, picking a deterministic object id from each — e.g., the first `*.container` declared in the example's services). Per sub-test assert exit 0, no writes to `/var/lib/core-ops/`, no audit files created when `--audit-dir` not set. This is the SC-011 coverage task — "any of the five published examples" requires all five exercised, not one.

### Implementation for User Story 1

> Authoring tasks T023–T027 are the validation work itself: research the upstream design from public sources, write own Quadlet equivalents (no verbatim YAML copy per research.md D5), and embed citations in each `README.md`. Each authoring task is one example, fully independent (different directory). Per example: ≥1 service definition, ≥1 host overlay, README with sources/intent/dispatch-table/known-limitations/Try-it-snippet.

- [X] T023 [P] [US1] Author `examples/01-caddy-whoami/`: single Container (Caddy fronting whoami), `services/<svc>/quadlet/`, default config-root, one host overlay, README citing Caddy docs + traefik/whoami container README.
- [X] T024 [P] [US1] Author `examples/02-nextcloud/`: multi-Container with Nextcloud + Postgres + Redis + Traefik (community multi-container — NOT Nextcloud AIO per research.md D5), intra-service `Network=`, persistent `Volume=`, `service.yaml` with `config-root` where ids diverge, host overlay with TLS/domain config in drop-ins, README citing Nextcloud's community docker-compose docs.
- [X] T025 [P] [US1] Author `examples/03-immich/`: server + db + redis + ML worker + Traefik, GPU device passthrough via `PodmanArgs=` or equivalent quadlet directive, multi-network membership (immich-internal + traefik network), host overlay, README citing `immich-app/immich` docker-compose.yml. Document any friction (e.g., NFS mount patterns) in README's `## Known limitations` and route to synthesis table.
- [X] T026 [P] [US1] Author `examples/04-traefik-authelia/`: Traefik + Authelia + protected backend (e.g., whoami), ForwardAuth middleware composition via Traefik labels (drop-ins on the protected backend), cross-service network, host overlay with auth domain configured, README citing Authelia's Traefik integration docs.
- [X] T027 [P] [US1] Author `examples/05-observability/`: Prometheus + Grafana + node-exporter + cadvisor, host-scope sidecars with `/proc` and `/sys` bind mounts (declared as `Volume=/proc:/host/proc:ro,rslave`-style mounts), scrape-config templating limitation captured in README + synthesis table, host overlay, README citing Prometheus/Grafana/node-exporter/cadvisor official compose examples.
- [X] T028 [US1] Register the five new test modules in `tests/integration/mod.rs`: `pub mod test_examples_01_caddy_whoami;` through `pub mod test_examples_05_observability;` plus `pub mod test_stateless_plan;` and `pub mod test_stateless_explain;` (single file → sequential).
- [X] T029 [P] [US1] Add `## Real-World Examples` section to repo-root `README.md` between `## First Interaction` and `## Installation (Current Phase)`, linking each of the five examples with a one-line purpose statement (single file → sequential within itself, parallel with all other US1 tasks).

**Checkpoint**: At this point, US1 is fully functional. A reviewer can run any of the five examples via stateless plan/explain. SC-001/003/006/007/008 measurable.

---

## Phase 4: User Story 2 — Operator authors and iterates on their own setup (Priority: P2)

**Goal**: Stateless mode supports non-git directories (FR-015) and the iterate-then-init transition is smooth (US2 AC3).

**Independent Test**: Copy an example to a scratch directory, modify it without `git init`, run `core-ops plan --source-repo <scratch> --host <host>` repeatedly with edits in between — all succeed. Then `git init && core-ops init` and verify subsequent `core-ops plan` (no flag) produces an equivalent plan.

### Tests for User Story 2 (REQUIRED) ⚠️

- [X] T030 [US2] Add to `tests/integration/test_stateless_plan.rs` (or a new `test_stateless_authoring.rs`): copy `examples/02-nextcloud/` via `copy_dir_recursive` helper to a tempdir, rename `hosts/example/` to `hosts/myhost/`, edit `host.yaml`, run `core-ops plan --source-repo <tempdir> --host myhost` → exit 0 (US2 AC1).
- [X] T031 [US2] Add a transition test: stateless-plan against a scratch dir, then `git init && git add . && git commit && core-ops init <scratch> main && core-ops plan` (no flag), assert the two plans produce equivalent action sets via `PlanOutput` JSON comparison (US2 AC3, idempotence under transition).

### Implementation for User Story 2

> US2's implementation surface is fully covered by the foundational stateless-mode wiring (T009–T012) plus the FR-015 non-git support already exercised in T021. No new code paths needed; T030–T031 are integration-test deliverables only.

- [X] T032 [US2] Verify each `examples/<NN-slug>/README.md` includes a "Scaffold for your own setup" section with explicit `cp -r examples/<NN-slug> ~/my-setup` instructions per quickstart.md Step 6. If absent, add to each README via a single edit pass (touches all five README files → sequential). Verified via `grep -l "## Scaffold for your own setup" examples/*/README.md` returning all five paths.

**Checkpoint**: US2 validated. Operators can use any example as a starting scaffold.

---

## Phase 5: User Story 3 — Stateless apply for one-off convergence and recovery (Priority: P2)

**Goal**: `core-ops apply --source-repo <PATH> --host <HOST>` mutates host state and writes path-based provenance; init'd `desired_state.*` is preserved (SC-009).

**Independent Test**: Stateless apply against a fresh host succeeds and produces audit + status with path-based provenance. Stateless apply against a host with prior init'd configuration leaves `desired_state.repository` and `desired_state.requested_ref` of the init'd config byte-identical pre/post.

### Tests for User Story 3 (REQUIRED) ⚠️

- [X] T033 [US3] Stateless apply integration test `tests/integration/test_stateless_apply.rs`: stateless apply against a synthetic source repo in tempdir; assert (a) exit 0, (b) audit record produced, (c) status snapshot reports `desired_state.repository = <canonical-path>`, `desired_state.requested_ref` matches expected sentinel/SHA per the source's git state (FR-013, US3 AC1, US3 AC2). Implementation interpretation: per FR-013 ("MUST converge host state and write audit records as today") + SC-009 ("init'd state byte-identical pre/post"), stateless apply writes audit records but does not write to /var/lib/core-ops/status.json. The "status snapshot" assertion in (c) is asserted against the audit-bundle's `result.desired.requested_repository`/`requested_ref` fields, which carry the canonical-path / sentinel-or-SHA values produced by `detect_provenance`.
- [X] T034 [US3] Add to `test_stateless_apply.rs` or new file: (a) **Init'd-state preservation test** — `core-ops init <synthetic-repo> main` (write init'd state to a `--state-file` tempdir), then `core-ops apply --source-repo <other-path> --host <host> --state-file <same-tempdir>`, assert `desired_state.repository` and `desired_state.requested_ref` from the init'd phase are byte-identical pre/post the stateless apply (SC-009). (b) **Stateless-apply → init'd-plan transition test** (US3 AC3) — after the stateless apply lands, run `core-ops init <synthetic-repo> main --force --state-file <same-tempdir>` then `core-ops plan --state-file <same-tempdir>` (no `--source-repo`); assert plan exits 0 and produces a normal init'd-mode plan with no detached-state header and no rollback ambiguity surfacing from the prior stateless apply.
- [X] T035 [US3] Add to `test_stateless_apply.rs`: provenance-shape coverage — three sub-cases asserting `(stateless)` / `(stateless+dirty)` / SHA recorded under three working-tree conditions (matches T021's plan-side coverage but for apply's persisted snapshot). Sequential within `test_stateless_apply.rs` after T033/T034.

### Implementation for User Story 3

> US3's implementation is fully covered by T011 (stateless wiring in apply.rs) and the provenance recording it lands. No new code paths needed; T033–T035 are integration-test deliverables.

- [X] T036 [US3] Register `pub mod test_stateless_apply;` in `tests/integration/mod.rs` (single file → sequential after T028).

**Checkpoint**: US3 validated. Stateless apply is functional with correct provenance and init'd-state preservation.

---

## Phase 6: User Story 4 — Synthesis table populated and reviewed (Priority: P3)

**Goal**: Friction-classification synthesis table in `spec.md` carries every translation finding with classification A/B/C, satisfying FR-005 and SC-002. Future spec authors have an evidence base.

**Independent Test**: Open `spec.md`, count rows in the `## Synthesis table` section, verify each row has all five required columns and Classification ∈ {A, B, C}, verify each `B`-row's workaround text exists in the affected example's README under `## Known limitations`, verify each `C`-row corresponds to a `docs/follow-ups.md` bullet, verify each `A`-row references a real follow-up spec number.

### Tests for User Story 4

> The synthesis table is markdown content, not code. Verification is review-time per contracts/synthesis-table.md invariants 1–5. No automated tests added.

### Implementation for User Story 4

- [ ] T037 [US4] Add an empty `## Synthesis table` section to `spec.md` between `## Success Criteria` and `## Assumptions`, with the column-header row only and an instruction comment for the synthesis pass: `<!-- Populated during translation (Phase 3 tasks T023–T027) and reviewed during this phase. See contracts/synthesis-table.md for invariants. -->`.
- [ ] T038 [US4] Synthesis review pass: read every `examples/<NN-slug>/README.md`'s `## Known limitations` section, transcribe each friction as a row in the synthesis table with the correct classification per contracts/synthesis-table.md semantics. The first row pre-populated by this slice is the stateless-mode self-escalation: `Stateless plan/apply/explain blocked all five examples (CLI gap, not layout gap) | 01..05 | A | Layout was sufficient; bottleneck was missing --source-repo CLI surface. Self-escalation absorbed in this slice per 2026-05-05 operator approval. | Escalate to spec/017 (this iteration absorbs the fix)`.
- [ ] T039 [US4] Verify synthesis-table invariants 1–5 from contracts/synthesis-table.md: every example's known-limitations entry is reflected; every `A` row references a real spec; every `B` row has its README workaround; every `C` row has a follow-up bullet. If any invariant fails, fix the table or the corresponding artifact.

**Checkpoint**: All four user stories complete. Validation iteration is structurally sound.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Stale-doc cleanup, release governance, final validation gates.

- [ ] T040 Stale-doc cleanup: `docs/follow-ups.md` lines 7–14 — remove the now-shipped paragraphs about `--repo`/`--rev` argument removal (FR-020). Preserve still-valid follow-ups in the same section: quadlet-dir/systemd-unit-dir/state-file/audit-dir arg persistence; rollback-plan-only re-homing; `--reinitialize` UX. Also remove (or amend) the two follow-up bullets at lines 87–99 (Source Repository UX → "Rich, documented real-life examples" and "QnA for known limitations") — closed by this slice.
- [ ] T041 [P] Stale-doc cleanup: `docs/development.md:228` — replace `CORE_OPS_HOST=<host> core-ops plan --repo <repo> --rev <rev>` with `core-ops plan --source-repo <PATH> --host <HOST>` plus a brief note about the init'd-mode workflow.
- [ ] T042 [P] Stale-doc cleanup: `infra/repo/README.md` lines 32, 35, 38 — update each `core-ops plan --repo file:///… --rev demo-uat-vN` to use `--source-repo <PATH>` against the demo repo's checkout.
- [ ] T043 Update `Cargo.toml` version: bump from current master `2.1.1` to next minor. Run `cargo run --bin core-ops-release -- validate --base-ref master` first to confirm whether the validator infers `minor` or `major` from spec/016 example deletions; bump to `2.2.0` if minor, `3.0.0` if major (per FR-021).
- [ ] T044 Add release fragment `changes/017-real-world-validation.md` declaring `release_intent: minor` (or `major` per T043's validator verdict) and listing: (a) `examples/` directory with five real-world setups, (b) stateless `--source-repo` flag for plan/apply/explain, (c) spec/016 example removal, (d) stale-doc cleanup, (e) root README "Real-World Examples" section.
- [ ] T045 Re-render `CHANGELOG.md` via `cargo run --bin core-ops-release -- changelog --write`. The `[Unreleased]` block should now contain the spec/017 fragment content. Do NOT manually author a `## [<version>]` section; the post-merge `core-ops-release promote` step (shipped v2.1.0) handles that automatically.
- [ ] T046 Run final `cargo build --locked --bin core-ops --bin core-ops-verify --bin core-ops-release` plus `cargo test` plus `cargo clippy --all-targets -- -D warnings`. ALL MUST PASS. This is the merge gate.
- [ ] T047 Run `cargo run --bin core-ops-release -- validate --base-ref master` — release governance gate must pass with the declared release-intent matching or exceeding the validator's inferred bump.
- [ ] T048 Run quickstart.md validation manually: execute Steps 1–6 against each of the five examples; confirm acceptance check items at end of quickstart.md all pass.
- [ ] T049 Privacy + RFC-compliance gate: (a) `grep -rE 'not\.one|ulthar|192\.168\.1\.2|gcloud[-_]dns|gcloud\.json' examples/` MUST return zero matches (SC-005, leaked operator-private values). (b) RFC 2606 compliance: every fully-qualified hostname under `examples/` MUST end in one of `.example.com`, `.example.org`, `.example.net`, `.test`, `.invalid`, `.localhost`, or be an unqualified service name (Quadlet container hostnames, intra-network references). (c) RFC 5737 compliance: every IPv4 literal MUST fall in `192.0.2.0/24`, `198.51.100.0/24`, or `203.0.113.0/24` (FR-008). Implementer crafts the exact grep/lint pattern; non-compliant hosts/IPs MUST be rewritten before merge.
- [ ] T050 Spec/017 self-update: tick off completed tasks in this `tasks.md` file as work lands, per the `feedback_speckit_tasks_checklist.md` discipline. Do not batch the ticks — update per task as it ships.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup, T001–T005)**: No dependencies. T001 can run first; T002 follows T001; T003–T005 parallel with each other after T001.
- **Phase 2 (Foundational, T006–T015)**: Depends on Phase 1 completion. **BLOCKS all user stories.** T006 → T007 → T008 (same file `args.rs`); T009 / T010 / T011 / T012 parallel after T006–T008; T013 sequential after T006–T008; T014 parallel with T009; T015 (validation gate) sequential after all above.
- **Phase 3 (US1, T016–T029)**: Depends on Phase 2 completion. T016–T020 (per-example tests) parallel; T023–T027 (per-example authoring) parallel and independent of T016–T020 ordering — but each test passes only after its example exists. T021 / T022 (stateless plan/explain integration tests) parallel with T016–T020. T028 (mod.rs registration) sequential after T016–T022. T029 (root README) parallel with everything else.
- **Phase 4 (US2, T030–T032)**: Depends on Phase 2 + at least one example from Phase 3 (T024 specifically for T030). T030 / T031 parallel with each other if they live in different files; T032 single-file pass → sequential.
- **Phase 5 (US3, T033–T036)**: Depends on Phase 2 (T011 specifically). T033 / T034 / T035 are all in `test_stateless_apply.rs` and therefore sequential within that file (no `[P]`); T036 (mod.rs registration) sequential after T035 and after T028 (single-file edits to `tests/integration/mod.rs`).
- **Phase 6 (US4, T037–T039)**: Depends on Phase 3 completion (synthesis review needs all examples authored). T037 → T038 → T039 sequential (each operates on `spec.md`).
- **Phase 7 (Polish, T040–T050)**: Depends on Phase 3, 4, 5, 6 completion. T040 / T041 / T042 parallel; T043 → T044 → T045 sequential (release governance pipeline); T046 → T047 sequential after T043–T045; T048 / T049 parallel; T050 cross-cutting.

### User Story Dependencies

- **US1 (P1)**: After Foundational (Phase 2). Independent of US2/US3/US4.
- **US2 (P2)**: After Foundational + at least one US1 example (T024 specifically). Independent of US3.
- **US3 (P2)**: After Foundational. Independent of US1/US2.
- **US4 (P3)**: After all US1 example READMEs are authored (T023–T027). Independent of US2/US3 except for transcribed friction.

### Parallel Opportunities

- **Within Phase 1**: T003 / T004 / T005 parallel after T001 (different files).
- **Within Phase 2**: T009 / T010 / T011 / T012 parallel after T006–T008 land. T014 parallel with T009.
- **Within Phase 3**: T016–T020 parallel; T023–T027 parallel; T021 / T022 parallel.
- **Across user stories**: With multiple developers, US1 / US3 can run fully in parallel after Phase 2.
- **Within Phase 7**: T040 / T041 / T042 parallel.

---

## Parallel Examples

### Phase 2 foundational parallelism (after T006–T008 land args.rs)

```bash
# Launch in parallel:
Task: "T009 Implement src/io/source_ref.rs::detect_provenance"
Task: "T010 Wire stateless source resolution into src/cli/plan.rs"
Task: "T011 Wire stateless source resolution into src/cli/apply.rs"
Task: "T012 Wire stateless source resolution into src/cli/explain.rs"
Task: "T014 Unit tests for src/io/source_ref.rs"
```

### US1 example authoring (after Phase 2 checkpoint)

```bash
# Launch in parallel:
Task: "T023 Author examples/01-caddy-whoami/"
Task: "T024 Author examples/02-nextcloud/"
Task: "T025 Author examples/03-immich/"
Task: "T026 Author examples/04-traefik-authelia/"
Task: "T027 Author examples/05-observability/"
```

### US1 per-example integration tests (parallel with authoring)

```bash
# Launch in parallel (after each example's authoring lands):
Task: "T016 Per-example test for examples/01-caddy-whoami"
Task: "T017 Per-example test for examples/02-nextcloud"
Task: "T018 Per-example test for examples/03-immich"
Task: "T019 Per-example test for examples/04-traefik-authelia"
Task: "T020 Per-example test for examples/05-observability"
```

---

## Implementation Strategy

### MVP First (US1 only — runnable real examples)

1. Phase 1: Setup — clear spec/016 examples and their consumer.
2. Phase 2: Foundational — land `--source-repo` across plan/apply/explain plus the source_ref helper.
3. Phase 3: US1 — author the five examples and their integration tests; ship root README section.
4. **STOP and VALIDATE**: Run `core-ops plan --source-repo examples/01-caddy-whoami --host example` end-to-end; confirm all five examples parse and run. SC-001, SC-003, SC-006, SC-007, SC-008 measurable.
5. (Optional) Demo: a reviewer who has never seen CoreOps clones the repo and runs an example in under 5 minutes.

### Incremental Delivery

1. MVP (Setup + Foundational + US1) → first valuable increment, addresses the v2.0.0 broken-examples regression.
2. Add US3 → stateless apply with provenance preservation. Critical for the recovery and CI workflows in spec.md.
3. Add US2 → non-git authoring scaffolds and stateless-to-init'd transition test. Mostly tests, no new implementation.
4. Add US4 → synthesis table population. Closes the validation iteration's evidence loop.
5. Polish → stale-doc cleanup, release governance, privacy gate, quickstart validation.

### Parallel Team Strategy

- Developer A: Phase 2 foundational (T006–T015), unblocks everyone.
- Developer B: After T015 lands, takes US1 example authoring (T023–T027 in parallel).
- Developer C: After T015 lands, takes US3 stateless apply tests (T033–T036).
- Developer D: After all US1 README authoring lands, takes US4 synthesis review (T037–T039).
- Polish phase shared at the end.

---

## Notes

- **Tests are mandatory** per FR-006, FR-016, and the spec's Constitution Alignment. The VM-backed-scenario exemption is recorded in spec.md and plan.md per Principle 10.
- **Tick off `- [X]` per task as it ships** per `feedback_speckit_tasks_checklist.md` — not batched at session end. Keep `tasks.md` in sync with git log.
- **Conventional commit messages** per `feedback_commit_style.md`: `feat(scope): subject` for code, `docs(scope): subject` for docs, `test(scope): subject` for tests. No `[Spec Kit]` prefix.
- **`cargo clippy --all-targets -- -D warnings` after every Rust-touching task** per `feedback_clippy.md`, before commit.
- **`--source-repo` flag naming is locked** per spec.md and contracts/cli-flag.md. Do not bikeshed.
- **No real-world ulthar values in `examples/`** per FR-009. Privacy gate at T049.
- **License hygiene**: write own Quadlet equivalents inspired by upstream sources. Cite upstream in each example README under a `## Sources` heading. Do NOT copy YAML blocks verbatim (research.md D5).
- **Spec/016 example removal happens BEFORE spec/017 examples are authored** (T001 in Phase 1) so `tests/integration/source_repo_support.rs:20` doesn't reference deleted dirs while implementation is in flight.
- **Self-escalation row in synthesis table** (T038) is the only `A`-classified-with-absorption row allowed; any other in-scope absorption is scope creep.
- **Avoid**: vague tasks, same-file conflicts, cross-story implementation dependencies that break independence.
