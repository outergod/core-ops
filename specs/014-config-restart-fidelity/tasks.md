# Tasks: Config Change Restart Fidelity

**Input**: Design documents from `specs/014-config-restart-fidelity/`
**Prerequisites**: plan.md ✅ spec.md ✅ research.md ✅ data-model.md ✅ quickstart.md ✅

**Tests**: Included. Standard validation gates apply: `cargo test` and
`cargo clippy --all-targets -- -D warnings`.

**Organization**: Tasks are grouped by user story to enable independent
implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to

---

## Phase 1: Setup

**Purpose**: Confirm branch, locate affected code.

- [ ] T001 Verify working branch is `014-config-restart-fidelity` (`git branch --show-current`)
- [ ] T002 [P] Read `src/core/planner.rs` — study `plan()` (lines 17–83) and `actions_for_diff` (lines 626–730) to understand the diff-to-action mapping and where the ConfigFile gap is
- [ ] T003 [P] Read `src/core/evaluate.rs` — study `dependency_refs_for_workload_state` (lines 101–168) to confirm it parses `EnvironmentFile=` directives against `managed_config_paths` and returns path-based refs

---

## Phase 2: Foundational

**Purpose**: Confirm the shared key identity (`systemd_unit_name == target_path == dependency_ref`) and the existing test harness shape before writing any code.

- [ ] T004 Read `tests/integration/test_plan.rs` around the existing `desired_snapshot_extracts_config_and_runtime_dependency_refs` test (line 392) to understand how `DesiredState`, `Workload`, and `ObservedState` are constructed in tests — this is the exact shape the regression tests will use
- [ ] T005 [P] Read `src/io/repo.rs:591` (`workload_from_config_file`) to confirm `systemd_unit_name = file.target_path` — this validates the key identity assumption from research.md D1
- [ ] T006 [P] Read `src/cli/report.rs` — locate `build_apply_output` (line 476) and `convergence_failed_for_entry` (line 1374) to confirm the P2 resolution path: failed RestartUnit → `failed_actions` → `convergence_failed_for_entry` → terminal `Failed` in report

**Checkpoint**: Foundation confirmed — ready to write failing tests and implement.

---

## Phase 3: User Story 1 — Config change triggers actual service restart (P1) 🎯 MVP

**Goal**: `plan()` emits `RestartUnit` for containers whose config file changes,
is removed, or is added when the container is already running.

**Independent Test**: `cargo test config_file_change_schedules_restart` — the
test constructs `DesiredState` + `ObservedState` with a changed config file and
asserts `RestartUnit` appears in `ReconciliationPlan.actions`.

### Tests for US1 (write first — MUST FAIL before T011)

- [ ] T007 [P] [US1] Add test `config_file_change_schedules_restart_for_dependent_container` to `tests/integration/test_plan.rs`: desired has ConfigFile (new contents) + Container (EnvironmentFile dep); observed has both (old config contents + container present); assert `actions` contains `RestartUnit` for the container and that `WriteQuadlet` for the config file precedes it
- [ ] T008 [P] [US1] Add test `config_file_change_no_restart_when_no_dependents` to `tests/integration/test_plan.rs`: desired has ConfigFile only (no dependent container); observed has old contents; assert no `RestartUnit` actions in plan
- [ ] T009 [P] [US1] Add test `config_file_remove_schedules_restart_for_dependent_container` to `tests/integration/test_plan.rs`: desired has Container only (config file removed); observed has both config file and container; assert `RestartUnit` for the container is scheduled
- [ ] T010 [P] [US1] Add test `config_file_change_no_duplicate_restart_when_container_also_changed` to `tests/integration/test_plan.rs`: both config file and container contents changed; assert exactly one `RestartUnit` for the container (not two)
- [ ] T011 Run `cargo test config_file_change config_file_remove` and confirm all four tests FAIL (required before implementing)

### Implementation for US1

- [ ] T012 [US1] In `src/core/planner.rs`, add `use crate::core::evaluate::dependency_refs_for_workload_state;` import and a dependent-restart pass after the main `for diff in &diffs` loop in `plan()`:
  - Build `observed_unit_names: HashSet<String>` from `observed.workloads`
  - Build `already_restarted: HashSet<String>` from existing `actions` (any `PlanActionType::RestartUnit` targets)
  - For each diff where `quadlet_type == QuadletType::ConfigFile` and `kind` is `Add | Change | Remove`:
    - For each workload in `desired.workloads`:
      - Compute `deps = dependency_refs_for_workload_state(desired, workload)`
      - If `deps.contains(&diff.name)`:
        - For `Add`: restart only if `observed_unit_names.contains(&workload.systemd_unit_name)`
        - For `Change | Remove`: always restart
        - Skip if `already_restarted` contains the unit name
        - Push `action(PlanActionType::RestartUnit, &workload.systemd_unit_name)` and insert into `already_restarted`
- [ ] T013 [US1] Run `cargo test config_file_change config_file_remove` and confirm all four US1 tests now PASS
- [ ] T014 [US1] Run `cargo test` (full suite) and confirm no regressions

**Checkpoint**: US1 complete — containers now actually restart when their config files change.

---

## Phase 4: User Story 2 — Apply report reflects actual execution (P2)

**Goal**: Verify that when a `RestartUnit` is present in the executable plan,
apply report terminal state reflects real execution outcome (restarted on
success, failed on failure). No new production code required; P1's fix closes
the report gap.

**Independent Test**: The US1 tests (T007–T010) already validate that the
planner emits `RestartUnit` — which is the prerequisite for accurate reporting.
The US2 test below validates that a failed restart surfaces correctly.

### Validation for US2

- [ ] T015 [P] [US2] Add test `config_file_change_report_shows_restarted_from_execution` to `tests/integration/test_plan.rs`: call `build_apply_output` from `src/cli/report.rs` with the deterministic plan for a config-file change scenario; assert the container's terminal `ExecutionEvent.state` is `Restarted` (not `Unchanged`) — confirming the report is consistent with the now-executed restart
- [ ] T016 [P] [US2] Read `src/cli/report.rs:476–596` (`build_apply_output`) and `src/core/verify.rs:61–70` and confirm in code comments or a note that no additional report-sourcing change is needed: after P1, the terminal state is `Restarted` (synthesised from plan action) and, if the service fails to restart, `verify_state` will populate `failed_actions`, causing `convergence_failed_for_entry` to return `true` and the report to show `Failed`
- [ ] T017 [US2] Run `cargo test config_file_change_report` and confirm T015 passes

**Checkpoint**: US2 validated — apply report accurately reflects restart execution.

---

## Phase 5: User Story 3 — Regression test coverage for Add cases (P3)

**Goal**: Full regression suite covering the `DiffKind::Add` edge cases (new
config file with pre-existing container vs. new container).

**Independent Test**: `cargo test config_file_add` — both tests run without
requiring systemd or filesystem access.

### Tests for US3 (write first — MUST FAIL before T020 if T012 not yet applied, verify pass if T012 was applied)

- [ ] T018 [P] [US3] Add test `config_file_add_restarts_already_running_container` to `tests/integration/test_plan.rs`: desired has config file (new) + container (EnvironmentFile dep); observed has container PRESENT but config file ABSENT; assert `RestartUnit` for the container is scheduled (container was running without the config file, now needs to pick it up)
- [ ] T019 [P] [US3] Add test `config_file_add_no_restart_for_new_container` to `tests/integration/test_plan.rs`: desired has config file (new) + container (EnvironmentFile dep); observed has NEITHER; assert NO `RestartUnit` for the container (fresh `StartUnit` from container's own diff is sufficient)
- [ ] T020 [US3] Run `cargo test config_file_add` and confirm both tests pass (they should, since the planner pass from T012 handles the Add case with the observed-state discriminator)

**Checkpoint**: US3 complete — Add-case edge cases covered and regression-proof.

---

## Phase 6: Polish & Release Governance

**Purpose**: Validation gates, release artefacts, and changelog.

- [ ] T021 Run `cargo clippy --all-targets -- -D warnings` and fix any new warnings introduced by T012
- [ ] T022 Run `cargo test` (full suite) and confirm all tests pass, no regressions
- [ ] T023 [P] Create `changes/014-config-restart-fidelity.md` with `release_intent: patch`, `scope: planner`, `release_preparation: false`, and summary: "Fix config-file changes not triggering dependent container restarts"
- [ ] T024 [P] Bump version in `Cargo.toml` to `0.8.2` (patch bump from current `0.8.1`)
- [ ] T025 Update `CHANGELOG.md`: add `## [0.8.2]` section (or update `[Unreleased]`) with a `### Fixed` entry covering: (a) config-file changes now restart dependent containers; (b) config-file removal and addition with pre-existing containers also trigger restarts
- [ ] T026 Run `cargo run --bin core-ops-release -- validate --base-ref HEAD^` and confirm governance check passes (fragment present, version bumped, CHANGELOG updated)
- [ ] T027 Run the quickstart validation from `specs/014-config-restart-fidelity/quickstart.md` if a live host is available, or confirm via `cargo test` that SC-001 through SC-005 are satisfied

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies — start immediately
- **Phase 2 (Foundational)**: Parallel with Phase 1 after T001 — confirms assumptions before coding
- **Phase 3 (US1)**: Requires Phase 1 & 2 complete; T007–T010 (tests) before T012 (implementation)
- **Phase 4 (US2)**: Requires T012 (planner fix) complete; T015–T016 are parallel
- **Phase 5 (US3)**: Requires T012 (planner fix) complete; parallel with Phase 4
- **Phase 6 (Polish)**: Requires all user story phases complete

### User Story Dependencies

- **US1 (P1)**: Depends on Phase 2 only — independent
- **US2 (P2)**: Depends on US1 (T012) — P1 fix is the prerequisite
- **US3 (P3)**: Depends on US1 (T012) — Add-case tests need the planner pass in place

### Within Each Phase

- Tests MUST be written and FAIL (T011) before implementation (T012)
- Implementation (T012) before validation runs (T013, T014)

### Parallel Opportunities

- T002, T003 (Phase 1 reads) can run in parallel
- T004, T005, T006 (Phase 2 reads) can run in parallel
- T007, T008, T009, T010 (US1 failing tests) can be written in parallel
- T015, T016 (US2 validation) can run in parallel
- T018, T019 (US3 Add-case tests) can run in parallel
- T023, T024 (fragment + version bump) can run in parallel

---

## Parallel Example: User Story 1 Tests

```bash
# Write all four failing US1 tests in parallel (different test functions, same file):
Task: "config_file_change_schedules_restart_for_dependent_container"
Task: "config_file_change_no_restart_when_no_dependents"
Task: "config_file_remove_schedules_restart_for_dependent_container"
Task: "config_file_change_no_duplicate_restart_when_container_also_changed"

# Then a single implementation task (T012) makes them all pass.
```

---

## Implementation Strategy

### MVP (US1 only)

1. Phase 1 + 2: Read and confirm assumptions
2. Phase 3: Write failing tests → implement `plan()` pass → confirm tests pass
3. **STOP and VALIDATE**: `cargo test` full suite — if passing, this already fixes the reported production incident
4. Phase 6 (governance): Release fragment + version bump + CHANGELOG

### Full Delivery

1. MVP above
2. Phase 4: US2 report-accuracy validation (no code change, just confirm)
3. Phase 5: US3 Add-case tests
4. Phase 6: Polish + governance

---

## Notes

- Total tasks: 27
- Tasks per user story: US1=8, US2=3, US3=3, Polish=7
- Parallel opportunities: T002/T003, T004/T005/T006, T007/T008/T009/T010, T015/T016, T018/T019, T023/T024
- MVP scope: Phase 1–3 + Phase 6 (US1 only, ~14 tasks)
- **No new types, no new modules** — all changes in `src/core/planner.rs` (T012) and `tests/integration/test_plan.rs` (T007–T010, T015, T018–T019)
- All regression tests are pure unit tests — no systemd, no filesystem, no temp dirs required
