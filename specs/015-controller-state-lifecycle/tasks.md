# Tasks: Controller State Model and Lifecycle

**Input**: Design documents from `specs/015-controller-state-lifecycle/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Rust validation gates (`cargo test` + `cargo clippy --all-targets -- -D warnings`) are required at the end of each phase. Unit tests for key invariants are included per phase. No new VM-backed scenarios are required for this feature's new behaviors; existing VM scenarios are updated as part of Phase 2 foundational work.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no unresolved dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup

**Purpose**: No new project scaffolding is needed. This feature extends an existing Rust binary with new source files and type changes.

- [X] T001 Confirm `cargo build --locked --bin core-ops` passes clean before starting work

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Type changes and structural rewiring that every user story depends on. Must be complete before any US phase begins. These changes are compile-time breaking — nothing compiles cleanly until callers are updated.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Add `#[error("state file is corrupt: {0}")] Corrupt(String)` variant to `StateError` enum in `src/core/errors.rs`
- [X] T003 Update `read_persisted_state` in `src/io/state.rs` to return `Err(StateError::Corrupt(path.display().to_string()))` when the file exists but `parse_persisted_state_text` returns `None` (absent → `Ok(None)` unchanged; only the invalid-file path changes)
- [X] T004 Add `#[serde(default)] pub detached: bool` field to `PersistedProvenanceState` in `src/core/types.rs`
- [X] T005 Add `InitArgs` struct (positional `repository: String`, positional `requested_ref: String`, `--force: bool`) and `Commands::Init(InitArgs)` to `src/cli/args.rs`; remove `--repo`/`--rev` from `PlanArgs`, `ApplyArgs`, `AgentArgs`, `ExplainArgs`; update all four `--after_help` strings to remove examples that use `--repo`/`--rev`
- [X] T006 Add `Init` variant (`#[serde(rename = "init")]`) to `VerificationCoreOpsActionKind` in `src/core/verification_model.rs`; add `#[serde(default)] pub force: bool` to `VerificationCoreOpsAction`; make `repository_source` and `revision` fields `#[serde(default)]`; update `render_coreops_action` to: emit `core-ops init <repo> <ref> [--force]` for `Init`; remove `--repo`/`--rev` from `Apply`, `Plan`, `Explain`, `Agent` branches; update `action_label` to include `Init → "init"`
- [X] T007 Update all 9 existing accepted scenarios in `tests/fixtures/verification/scenarios/` to add `init` steps before the first `apply`; add `init --force` before the second `apply` in `accepted-layered-upgrade-transition.yaml`, `accepted-mount-removal-ordering.yaml`, and `accepted-config-change-restart.yaml` (see `contracts/scenario-runner-changes.md` for per-scenario table)
- [X] T008 Run `cargo build --locked --bin core-ops` and confirm it fails only on `Commands::Init` not being dispatched in `src/main.rs` (expected compile error at this stage); fix any unexpected compile errors in T002–T007; also run `cargo clippy --all-targets -- -D warnings` on the already-compiling files (errors.rs, types.rs, verification_model.rs) to catch structural issues early

**Checkpoint**: Foundation ready — all type changes are in place; user story phases can proceed

---

## Phase 3: User Story 1 — Initialize Controller on a New Host (Priority: P1) 🎯 MVP

**Goal**: Operators run `core-ops init <repository> <ref>` once; all reconciliation commands (`plan`, `apply`, `agent`, `explain`) then source repository and ref exclusively from persisted state without per-invocation arguments. Absent state produces a clear, actionable initialization error.

**Independent Test**: Run `core-ops init <repository> <branch>` with no prior state file; verify `core-ops status` shows initialized configuration; run `core-ops plan` without `--repo`/`--rev` and verify it proceeds using persisted state.

### Tests for User Story 1

- [X] T009 [P] [US1] Add unit test in `src/cli/init.rs` (or `tests/`) covering: (a) success on absent state writes `NeverRun` state with correct fields; (b) `init` without `--force` on valid existing state returns "already initialized" error; (c) `init` on corrupt state without `--force` returns "corrupt state" error (message contains file path); (d) ref validation rejects bare 40-hex commit hashes; (e) ref validation rejects `HEAD`; (f) ref validation accepts a branch name; (g) ref validation accepts a tag name; (h) `init --force` with same repo/ref preserves reconciliation history and clears detached flag; (i) `init --force` with different repo/ref resets reconciliation to NeverRun and clears retained snapshots; (j) `init` on a host where the state file parent directory does not exist creates the directory before writing; (k) deserialization of a state file JSON without `"detached"` field produces `detached == false` (backward compat guard for FR-018)
- [X] T010 [P] [US1] Add unit test verifying that `run_agent` with no state file returns a `CoreError` with a message directing the operator to `core-ops init` (not the old bootstrap path)

### Implementation for User Story 1

- [X] T011 [P] [US1] Create `src/cli/init.rs` — implement `InitConfig` struct (repository, requested_ref, force, state_file override) and `run_init(config: &InitConfig) -> Result<(), CoreError>` following the contract in `contracts/cli-init.md`: read state, check for existing config, validate ref against repo, write `PersistedProvenanceState` with `NeverRun` reconciliation and `detached = false`; on `--force` with unchanged repo/ref preserve existing reconciliation and deterministic state (retained snapshots), clear detached flag only; on `--force` with changed repo/ref reset reconciliation to `NeverRun` and clear retained snapshots from `deterministic-state.json` (see `contracts/cli-init.md` Reinitialization Rules — changed tracking config discards prior snapshots); also create parent directory if absent
- [X] T012 [P] [US1] Update `src/cli/agent.rs` — remove `repo: String` and `rev: String` from `AgentConfig`; replace `persist_never_run_state` bootstrap block with lifecycle check: `Ok(None)` → fail with "controller not initialized" error; `Err(StateError::Corrupt(_))` → fail with "state file corrupt" error; `Ok(Some(state))` where `state.detached` → handled in US2; `Ok(Some(state))` otherwise → read `state.desired_state.repository` and `state.desired_state.requested_ref` for reconciliation; remove `CORE_OPS_REPO`/`CORE_OPS_REV` env var resolution
- [X] T013 [P] [US1] Update `src/cli/plan.rs` — remove `repo`/`rev` parameters from the plan invocation path; read `desired_state.repository` and `desired_state.requested_ref` from persisted state; fail with "controller not initialized" error on `Ok(None)` and "state file corrupt" error on `Err(StateError::Corrupt(_))`
- [X] T014 [P] [US1] Update `src/cli/apply.rs` — remove `repo`/`rev` parameters from the non-rollback apply path; read from persisted state; fail with lifecycle errors on absent/corrupt; the rollback path and detached-mode guard are handled in US2
- [X] T015 [P] [US1] Update `src/cli/explain.rs` — remove `--repo`/`--rev` fallback from `resolve_explain_target`; read exclusively from `desired_state.repository` and `desired_state.requested_ref`; fail with lifecycle errors on absent/corrupt
- [X] T016 [US1] Update `src/main.rs` — add `Commands::Init(args)` dispatch calling `run_init`; remove `resolve_env(args.repo, "CORE_OPS_REPO")` and `resolve_env(args.rev, "CORE_OPS_REV")` for all four affected command branches; audit the `.ok().flatten()` call at line 161 (post-apply audit — confirm it remains `.ok().flatten()` as best-effort only, no change needed)
- [X] T017 [US1] Update `src/cli/status.rs` — add `Err(StateError::Corrupt(_))` arm to the `read_persisted_state` match and report corrupt state clearly (path + recovery hint); expose in status output: (a) FR-016 fields: `desired_state.repository`, `desired_state.requested_ref`, `desired_state.last_observed_revision`, `reconciliation.last_applied_revision`; (b) derived **lifecycle state** label (Uninitialized / Corrupt / Initialized / Reconciling / Converged / Diverged / Detached) per the contract in `contracts/cli-command-changes.md`; (c) currently applied detached revision when lifecycle state is Detached (FR-017, deferred to T024 for the Detached-specific display logic, but the lifecycle label must be present here)
- [X] T018 [US1] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; fix all warnings; confirm init, agent, plan, apply, explain unit tests pass

**Checkpoint**: `core-ops init` + all reconciliation commands work from persisted state; absent state produces actionable errors

---

## Phase 4: User Story 2 — Survive a Snapshot Rollback Without Losing Recovered State (Priority: P2)

**Goal**: A successful snapshot rollback sets `detached = true` before completing. In Detached state, `agent` runs on schedule, emits an observable status with the currently applied detached revision, and exits without performing reconciliation. Further rollback from Detached remains permitted and leaves the controller Detached. Re-attachment via `init --force` with same repo/ref preserves history.

**Independent Test**: Perform a snapshot rollback from Converged state; verify `status` shows Detached with the rolled-back revision; trigger `agent` and verify it exits cleanly (exit 0) without modifying managed units, and emits a detached status message naming the revision.

### Tests for User Story 2

- [X] T019 [P] [US2] Add unit tests for rollback apply path in `apply.rs`: (a) initial rollback from Converged writes `detached = true` and correct `last_applied_revision` before returning success; (b) **further rollback from Detached** (when `state.detached == true` on entry) also writes `detached = true` with the *new* revision as `last_applied_revision`, asserting `detached` is preserved and the revision changes (US2 acceptance scenario 3 / FR-013)
- [X] T019b [P] [US2] Add unit test for rollback eligibility that asserts FR-014: construct a `DeterministicPersistedState` with a retained snapshot for a revision that does not exist in the test repo fixture; call `resolve_rollback_target` with that revision; assert the result is `RollbackEligibility::Eligible` (not a Git-reachability rejection). This is a regression guard — if Git reachability is ever accidentally added to the eligibility check, this test will fail.
- [X] T020 [P] [US2] Add unit test for `run_agent` when `state.detached == true`: verify it returns without calling `apply_with_report`; verify it returns an `AgentOutput` or exits cleanly; verify emitted log/message contains "detached" and the applied revision

### Implementation for User Story 2

- [X] T021 [US2] Update the rollback apply path in `src/cli/apply.rs` — after a successful snapshot rollback apply, write `detached = true` and update `reconciliation.last_applied_revision` to the rolled-back revision in persisted state before returning success; ensure `desired_state.requested_ref` is unchanged
- [X] T022 [US2] Update `src/cli/agent.rs` — add detached-state check before the apply path: if `state.detached == true`, emit a message matching the contract in `contracts/error-messages.md` ("controller is detached at revision {revision}; …") and return/exit cleanly without calling `apply_with_report`
- [X] T023 [US2] Update `src/cli/apply.rs` — add a guard for the non-rollback apply path: if `state.detached == true`, fail with the detached-state error message; `apply --rollback-to` MUST still proceed from Detached (this is the further-rollback path already handled in T021)
- [X] T024 [US2] Update `src/cli/status.rs` — expose `detached` flag and currently applied detached revision (`reconciliation.last_applied_revision`) in status output when `state.detached == true` (FR-017)
- [X] T025 [US2] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; fix all warnings; verify agent detached path tests pass

**Checkpoint**: Snapshot rollback enters Detached state; agent skips reconciliation and reports clearly; status shows detached revision

---

## Phase 5: User Story 3 — Inspect What Re-Attaching Would Apply While Detached (Priority: P3)

**Goal**: While in Detached state, `core-ops plan` produces a normal plan using the currently applied detached revision as the baseline and the current `requested_ref` HEAD as the target, with a clearly visible Detached-mode header in the output.

**Independent Test**: With the controller in Detached state, run `core-ops plan`; verify the output includes a Detached-mode header, exit code is 0, and the plan baseline is the detached revision not the last-applied-before-rollback revision.

### Tests for User Story 3

- [X] T026 [P] [US3] Add unit/integration test for `plan` in Detached state: verify output contains the detached header from `contracts/error-messages.md`; verify plan baseline uses `reconciliation.last_applied_revision` (the detached revision); verify exit code 0

### Implementation for User Story 3

- [X] T027 [US3] Update `src/cli/plan.rs` — detect `state.detached == true` after reading persisted state; prepend the detached-mode header from `contracts/error-messages.md` to plan output; plan otherwise behaves normally (resolves `requested_ref` to current HEAD, uses `last_applied_revision` as baseline — this is the existing three-way plan behavior)
- [X] T028 [US3] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; fix all warnings; verify Detached-mode plan test passes

**Checkpoint**: `core-ops plan` in Detached state is fully functional and clearly indicates Detached context

---

## Phase 6: User Story 4 — Recover from a Corrupt State File (Priority: P4)

**Goal**: When the state file exists but is corrupt, every CoreOps command fails with a distinct, named error that identifies the file path and provides `core-ops init <repository> <ref> --force` as the recovery command. This error must be visibly different from the absent-state (uninitialized) error.

**Independent Test**: Replace `/var/lib/core-ops/status.json` with malformed JSON; run `core-ops plan`; verify the error names the file path and directs to `--force`; run `core-ops plan` with the file removed; verify the error is visibly different.

**Note**: The `StateError::Corrupt` type and `read_persisted_state` behavior are already implemented in Phase 2 (T002–T003). This phase hardens and verifies the per-command error messages and the `init` corrupt handling added in T011.

### Tests for User Story 4

- [X] T029 [P] [US4] Add unit test for `read_persisted_state` directly: write a file with invalid JSON to a temp path; call `read_persisted_state`; assert result is `Err(StateError::Corrupt(_))` and the error string contains the file path; remove the file; assert result is `Ok(None)`
- [X] T030 [P] [US4] Add unit tests confirming each command (`agent`, `plan`, `apply`, `explain`) produces a visibly distinct error message for absent vs corrupt state: absent → message contains "not initialized" and names `core-ops init`; corrupt → message contains "corrupt" and names the file path and `--force`

### Implementation for User Story 4

- [X] T031 [US4] Audit every command's corrupt-state error arm (agent.rs, plan.rs, apply.rs, explain.rs, status.rs) and confirm the error message matches the canonical string in `contracts/error-messages.md`: `"state file at {path} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover"`; update any arm that uses a different message format or is missing the path; also verify rollback rejection messages for `IncompatibleScope` include both the snapshot's scope identifier and the current scope identifier (FR-015) matching the contract string: `"snapshot for revision {rev} was recorded on scope {snapshot_scope}, which is incompatible with current scope {current_scope}"`
- [X] T032 [US4] Confirm `src/cli/init.rs` (T011) handles `Err(StateError::Corrupt(_))` without `--force` with the canonical corrupt-state error message (not the "already initialized" message); add a test if not already covered by T009
- [X] T033 [US4] Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; confirm all corrupt-vs-absent distinction tests pass

**Checkpoint**: All commands produce distinct, named errors for absent vs corrupt state; recovery path is explicit in every error

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Release governance artifacts, final validation gate, documentation.

- [X] T034 Bump `version` in `Cargo.toml` from `"0.8.2"` to `"1.0.0"` (breaking CLI change: `--repo`/`--rev` removed from four commands)
- [X] T035 [P] Create `changes/015-controller-state-lifecycle.md` release-intent fragment declaring `bump: major` with rationale (removal of `--repo`/`--rev` from `plan`, `apply`, `agent`, `explain`)
- [X] T036 [P] Update `CHANGELOG.md` — add entry under `[Unreleased]` with `### Breaking Changes` (removal of `--repo`/`--rev` from four commands; operator migration: run `core-ops init` first), `### Added` (`core-ops init` command, Detached lifecycle state), `### Changed` (`agent` in Detached state exits without reconciling; `plan` in Detached state annotates output; `status` exposes repository/ref/lifecycle fields), `### Fixed` (corrupt state file now produces distinct named error instead of silent absent-state treatment)
- [X] T037 Run final `cargo test` and `cargo clippy --all-targets -- -D warnings` across the full change set; confirm zero warnings and zero test failures
- [X] T038 Verify `core-ops-verify` and `core-ops-release` binaries still build: `cargo build --locked --bin core-ops-verify --bin core-ops-release`
- [X] T039 Run `cargo run --bin core-ops-release -- validate --base-ref HEAD^` to confirm release-intent artifact is valid

---

## Phase 8: Promote Systemd Units to Static Assets

**Goal**: The canonical `core-ops.service` and `core-ops.timer` live in
`specs/002-systemd-agent/contracts/systemd/` and are outdated (reference the
now-removed `--repo`/`--rev` flags). Move them to a committed `systemd/`
directory at the repo root, update them to match the post-015 CLI, and fix all
references (CI, tests, README, quickstart).

- [X] T040 Create `systemd/core-ops.service` and `systemd/core-ops.timer` at the repo root with updated content: service uses `core-ops agent` with no `--repo`/`--rev`, drops `CORE_OPS_REPO`/`CORE_OPS_REV` env vars, adds a comment directing operators to run `core-ops init` first; timer is unchanged
- [X] T041 Update `tests/integration/test_systemd_units.rs` to reference `systemd/core-ops.service` and `systemd/core-ops.timer`
- [X] T042 Update `.github/workflows/ci.yml` to copy from `systemd/` instead of `specs/002-systemd-agent/contracts/systemd/`
- [X] T043 Update `README.md` and `specs/002-systemd-agent/quickstart.md` to reference `systemd/` and reflect post-015 operator flow (`core-ops init` then timer enable)
- [X] T044 Run `cargo test` and `cargo clippy --all-targets -- -D warnings`; confirm `systemd_unit_templates_exist` passes; confirm release validation still passes

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — **BLOCKS all user story phases**
- **US1 (Phase 3)**: Depends on Phase 2 — the core init + rewiring
- **US2 (Phase 4)**: Depends on Phase 3 — detached flag on rollback requires working apply path from US1
- **US3 (Phase 5)**: Depends on Phase 3 — detached plan header requires plan sourcing from state (US1); can run in parallel with US2
- **US4 (Phase 6)**: Depends on Phase 2 (StateError::Corrupt) and Phase 3 (command error messages written in US1); can run in parallel with US2 and US3
- **Polish (Phase 7)**: Depends on all user story phases

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational — no dependencies on other stories
- **US2 (P2)**: Depends on US1 (apply path must source from state before adding rollback detached behavior)
- **US3 (P3)**: Depends on US1 (plan must source from state); independent of US2
- **US4 (P4)**: Depends on Phase 2 and US1; independent of US2 and US3

### Parallel Opportunities Within US1 (Phase 3)

T009, T010 (tests), T011 (init.rs), T012 (agent.rs), T013 (plan.rs), T014 (apply.rs), T015 (explain.rs) can all run in parallel — they each touch different files. T016 (main.rs dispatch) must come after T011–T015 since it wires them together. T017 (status.rs) is independent.

### Parallel Opportunities Within Polish (Phase 7)

T034 (Cargo.toml), T035 (changes/), T036 (CHANGELOG.md) can all run in parallel — they are different files.

---

## Parallel Example: User Story 1 (Phase 3)

```
# All of these can launch simultaneously after Phase 2 completes:

T011: Create src/cli/init.rs
T012: Update src/cli/agent.rs (remove repo/rev, add lifecycle guard)
T013: Update src/cli/plan.rs (source from state)
T014: Update src/cli/apply.rs (source from state)
T015: Update src/cli/explain.rs (remove --repo/--rev fallback)
T017: Update src/cli/status.rs (expose new fields)

# Then after T011–T015 complete:
T016: Update src/main.rs (dispatch Init, remove env var resolution)

# Tests can draft in parallel with implementation:
T009: Unit tests for init.rs
T010: Unit test for agent absent-state behavior
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 2 (Foundational) — compile-breaking changes
2. Complete Phase 3 (US1) — `init` command + all four commands source from state
3. **STOP and VALIDATE**: `cargo test`, run `core-ops init` manually, run `core-ops plan` without flags
4. US1 is independently deployable and delivers the complete initialization UX

### Incremental Delivery

1. Phase 2 → Phase 3 (US1) → validate → foundation + init working
2. Phase 4 (US2) → validate → rollback survives agent timer
3. Phase 5 (US3) → validate → operator can inspect detached state
4. Phase 6 (US4) → validate → corrupt-state errors are distinct and actionable
5. Phase 7 → release governance → ready to merge

### Notes

- T003 (read_persisted_state change) is a compile-time-silent behavioral change — existing callers using `.map_err` will compile; callers with match arms on StateError variants need the Corrupt arm (status.rs). Do not skip the caller audit in T008.
- The two `.ok().flatten()` calls in `agent.rs:62` and `main.rs:161` are post-apply audit best-effort calls — they MUST remain `.ok().flatten()` and MUST NOT be changed to propagate the Corrupt error (they run after reconciliation completes).
- `init --force` with unchanged repo/ref MUST preserve `reconciliation.*` and `deterministic-state.json` (retained snapshots). This is an invariant tested in T009 and a source of subtle bugs if the implementation writes a fresh NeverRun state unconditionally.
- The scenario updates in T007 must precede running the E2E gate. The fixture repo revision values (`demo-uat-v2`, `config-v1`, etc.) are already tags in the fixture repos — no fixture repo changes are needed.
