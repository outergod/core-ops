---
description: "Task list for deterministic reconciliation"
---

# Tasks: Deterministic Reconciliation

**Input**: Design documents from `/specs/006-deterministic-reconcile/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g. US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add deterministic-reconciliation fixtures, test entry points, and contract validation scaffolding

- [X] T001 Create deterministic reconciliation fixture directory and scenario README in `tests/fixtures/deterministic_reconciliation/README.md`
- [X] T002 [P] Add baseline, external-drift, rollback, and oscillation fixture scenarios in `tests/fixtures/deterministic_reconciliation/`
- [X] T003 [P] Add three-way planning integration test scaffolding in `tests/integration/test_deterministic_planning.rs`
- [X] T004 [P] Add rollback integration test scaffolding in `tests/integration/test_rollback.rs`
- [X] T005 [P] Add convergence integration test scaffolding in `tests/integration/test_convergence.rs`
- [X] T006 Register deterministic reconciliation integration test modules in `tests/integration/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define shared state, persistence, and validation primitives required by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T007 Define normalized snapshot, dependency graph, planned action, drift, rollback, and convergence types in `src/core/types.rs`
- [X] T008 Add deterministic reconciliation, rollback, and non-convergence error variants in `src/core/errors.rs`
- [X] T009 Implement validation helpers for canonical object identity, semantic dependency cycles, rollback eligibility, and retry signatures in `src/core/validation.rs`
- [X] T010 Extend persisted state handling for retained successful snapshots, rollback metadata, and convergence records in `src/io/state.rs`
- [X] T011 Extend observed-state loading to produce normalized actual snapshots and runtime verification signals in `src/io/observed.rs`
- [X] T012 Extend desired-state evaluation to emit normalized managed objects and scope metadata in `src/core/evaluate.rs`
- [X] T013 Add three-way planner entry points and semantic dependency graph scaffolding in `src/core/planner.rs`
- [X] T014 [P] Add unit tests for deterministic reconciliation types and invariants in `tests/unit/test_types.rs`
- [X] T015 [P] Add unit tests for validation rules, cycle detection, and retry-signature invariants in `tests/unit/test_validation.rs`
- [X] T016 [P] Add unit tests for retained applied snapshot persistence and rollback retention behavior in `tests/unit/test_state_snapshot.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Explainable Three-Way Planning (Priority: P1) 🎯 MVP

**Goal**: Compute deterministic plans from desired, last applied, and actual state with explicit action classifications, dependency ordering, and structured diff output

**Independent Test**: Run planning repeatedly against identical desired, last-applied, and actual inputs and verify that CoreOps produces materially identical ordered machine-readable plans and human-readable explanations, including external drift and no-op cases.

### Tests for User Story 1 ⚠️

- [X] T017 [P] [US1] Add integration test for deterministic three-way planning and no-op detection across generated systemd units, Quadlet resources, managed mounts or automounts, and rendered host artifacts in `tests/integration/test_deterministic_planning.rs`
- [X] T018 [P] [US1] Add integration test for external drift classification and dependency-aware ordering in `tests/integration/test_plan.rs`
- [X] T019 [P] [US1] Add contract test for structured diff output in `tests/integration/test_status_contract.rs`
- [X] T020 [P] [US1] Add unit tests for action classification and deterministic ordering in `tests/unit/test_planner.rs`

### Implementation for User Story 1

- [X] T021 [US1] Implement normalized three-way semantic diffing and drift categorization across generated systemd units, Quadlet resources, managed mounts or automounts, and rendered host artifacts in `src/core/diff.rs`
- [X] T022 [US1] Implement deterministic action classification, topological ordering, and explanation generation across the supported managed resource kinds in `src/core/planner.rs`
- [X] T023 [US1] Implement planning orchestration for desired, last_applied, and actual state in `src/core/reconcile.rs`
- [X] T024 [US1] Implement machine-readable plan output and dry-run rendering in `src/cli/plan.rs`
- [X] T025 [US1] Derive human-readable plan rendering from structured diff output in `src/cli/report.rs`
- [X] T026 [US1] Surface three-way planning provenance, baseline revision, and drift summaries in `src/cli/status.rs`

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Safe Revision Rollback (Priority: P2)

**Goal**: Restore a previously successful retained revision through the normal planner with explicit eligibility checks, dependency-aware ordering, and partial-progress recording

**Independent Test**: Reconcile to one successful revision, advance to a later revision, then select the earlier retained revision and verify rollback planning and execution use the same planner, fail safely for ineligible revisions, and preserve the successful-apply boundary.

### Tests for User Story 2 ⚠️

- [X] T027 [P] [US2] Add integration test for rollback planning and execution against a retained successful revision in `tests/integration/test_rollback.rs`
- [X] T028 [P] [US2] Add integration test for rollback rejection when retained snapshot metadata is missing or expired in `tests/integration/test_rollback.rs`
- [X] T029 [P] [US2] Add integration test for partial rollback progress recording in `tests/integration/test_reconcile_apply.rs`
- [X] T030 [P] [US2] Add unit tests for rollback eligibility and successful-apply boundary rules in `tests/unit/test_invariants.rs`

### Implementation for User Story 2

- [X] T031 [US2] Implement rollback target resolution and retention-window eligibility checks in `src/io/state.rs`
- [X] T032 [US2] Implement rollback planning through the normal three-way reconciliation path in `src/core/reconcile.rs`
- [X] T033 [US2] Implement rollback-specific action ordering and disruption reporting in `src/core/planner.rs`
- [X] T034 [US2] Add rollback CLI arguments and execution wiring in `src/cli/args.rs` and `src/cli/apply.rs`
- [X] T035 [US2] Persist partial rollback outcomes without advancing `last_applied` in `src/io/audit.rs` and `src/io/state.rs`
- [X] T036 [US2] Surface rollback eligibility, target revision, and partial-progress results in `src/cli/status.rs` and `src/cli/report.rs`

**Checkpoint**: User Story 2 should be fully functional and testable independently

---

## Phase 5: User Story 3 - Non-Convergence Detection and Structured Reporting (Priority: P3)

**Goal**: Detect repeated failure, oscillation, and dependency blockage with bounded retries and expose consistent machine-readable and human-readable reconcile results

**Independent Test**: Simulate repeated failure, oscillation, and blocked prerequisites, then verify CoreOps stops after the bounded retry budget, records the affected objects and attempts, and emits matching machine-readable and human-readable result summaries.

### Tests for User Story 3 ⚠️

- [X] T037 [P] [US3] Add integration test for repeated-failure detection and bounded retry stop behavior in `tests/integration/test_convergence.rs`
- [X] T038 [P] [US3] Add integration test for oscillation detection across repeated reconciliation attempts in `tests/integration/test_retry.rs`
- [X] T039 [P] [US3] Add integration test for machine-readable and human-readable reconcile result parity in `tests/integration/test_apply_report.rs`
- [X] T040 [P] [US3] Add unit tests for convergence status classification and retry-budget enforcement in `tests/unit/test_verification.rs`

### Implementation for User Story 3

- [X] T041 [US3] Implement repeated-failure and oscillation signature tracking in `src/core/retry.rs`
- [X] T042 [US3] Implement post-apply convergence evaluation and non-convergence classification for the supported generated unit, Quadlet, mount or automount, and rendered artifact kinds in `src/core/verify.rs`
- [X] T043 [US3] Integrate bounded retry orchestration and intervention-required outcomes into `src/core/reconcile.rs` and `src/cli/agent.rs`
- [X] T044 [US3] Implement structured reconcile result output for apply and agent runs in `src/cli/report.rs` and `src/cli/apply.rs`
- [X] T045 [US3] Persist non-convergence diagnostics, affected objects, and attempt history summaries in `src/io/audit.rs` and `src/io/state.rs`
- [X] T046 [US3] Surface bounded retry exhaustion, oscillation, and blocked-state summaries in `src/cli/status.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, quickstart validation, release/version review, and full-suite regression coverage

- [X] T047 [P] Update deterministic reconciliation operator and developer guidance in `docs/development.md`
- [X] T048 [P] Update CLI help text and examples for three-way plan, rollback, and non-convergence reporting in `src/cli/args.rs`
- [X] T049 [P] Validate deterministic reconciliation quickstart scenarios in `tests/integration/test_quickstart_validation.rs`
- [X] T050 [P] Add end-to-end regression coverage for plan/apply/status provenance under deterministic reconciliation in `tests/integration/test_reconcile_provenance.rs`
- [X] T051 [P] Validate structured diff and rollback contracts against implemented behavior in `tests/integration/test_status_contract.rs` and `tests/integration/test_rollback.rs`
- [X] T052 [P] Document per-resource normalization rules and tolerated runtime variance for supported managed resource kinds in `specs/006-deterministic-reconcile/contracts/structured-diff.md` and `docs/development.md`
- [X] T053 [P] Record release-version-policy review outcome for deterministic reconciliation in `specs/006-deterministic-reconcile/plan.md`
- [X] T054 [P] Evaluate and apply the required controller package-version update in `Cargo.toml`
- [X] T055 Run the full deterministic reconciliation test suite with `cargo test`
- [X] T056 [P] Validate representative rollback plan and execution timing against SC-003 in `tests/integration/test_rollback.rs`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - MVP slice
- **User Story 2 (Phase 4)**: Depends on User Story 1 because rollback builds on the three-way planner and retained successful snapshots
- **User Story 3 (Phase 5)**: Depends on User Story 1 because bounded non-convergence is evaluated on top of the deterministic planning/apply path
- **Polish (Phase 6)**: Depends on all implemented user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: First deliverable after Foundational; no dependency on other stories
- **User Story 2 (P2)**: Requires User Story 1 planning, persisted snapshots, and structured plan output
- **User Story 3 (P3)**: Requires User Story 1 planning and apply/reporting flow; can proceed independently of rollback once that base exists

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Core state and planner changes before CLI/reporting changes
- Persistence changes before rollback or convergence outcomes rely on them
- Machine-readable output before human-readable rendering
- Story checkpoint must pass before advancing to the next dependent story

### Parallel Opportunities

- T002, T003, T004, and T005 can run in parallel after T001
- T014, T015, and T016 can run in parallel once the foundational types and state shape are drafted
- T017, T018, T019, and T020 can run in parallel for US1
- T027, T028, T029, and T030 can run in parallel for US2
- T037, T038, T039, and T040 can run in parallel for US3
- T047, T048, T049, T050, T051, T052, T053, and T054 can run in parallel in Polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 tests together:
Task: "Add integration test for deterministic three-way planning and no-op detection in tests/integration/test_deterministic_planning.rs"
Task: "Add integration test for external drift classification and dependency-aware ordering in tests/integration/test_plan.rs"
Task: "Add contract test for structured diff output in tests/integration/test_status_contract.rs"
Task: "Add unit tests for action classification and deterministic ordering in tests/unit/test_planner.rs"
```

## Parallel Example: User Story 2

```bash
# Launch User Story 2 rollback tests together:
Task: "Add integration test for rollback planning and execution against a retained successful revision in tests/integration/test_rollback.rs"
Task: "Add integration test for rollback rejection when retained snapshot metadata is missing or expired in tests/integration/test_rollback.rs"
Task: "Add unit tests for rollback eligibility and successful-apply boundary rules in tests/unit/test_invariants.rs"
```

## Parallel Example: User Story 3

```bash
# Launch User Story 3 convergence tests together:
Task: "Add integration test for repeated-failure detection and bounded retry stop behavior in tests/integration/test_convergence.rs"
Task: "Add integration test for oscillation detection across repeated reconciliation attempts in tests/integration/test_retry.rs"
Task: "Add integration test for machine-readable and human-readable reconcile result parity in tests/integration/test_apply_report.rs"
Task: "Add unit tests for convergence status classification and retry-budget enforcement in tests/unit/test_verification.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Confirm deterministic three-way planning, no-op detection, and structured diff output
5. Demo the plan-only MVP before moving to rollback and convergence behaviors

### Incremental Delivery

1. Complete Setup + Foundational -> deterministic planning foundation ready
2. Add User Story 1 -> Test independently -> Demo MVP
3. Add User Story 2 -> Test independently -> Demo retained-snapshot rollback
4. Add User Story 3 -> Test independently -> Demo bounded non-convergence and structured result reporting
5. Finish Polish and full-suite validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 planning, ordering, and structured diff output
   - Developer B: User Story 2 rollback eligibility, execution, and reporting
   - Developer C: User Story 3 retry, convergence, and result-status reporting
3. Merge when each story reaches its independent checkpoint

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps each task to a specific user story for traceability
- Each user story is independently testable from plan/apply/status behavior
- Tests are mandatory for this feature
- Include provenance/status assertions wherever persisted applied state or reconcile outcome semantics change
- Include release-version-policy updates because this feature changes externally observable reconciliation behavior, persisted state, CLI output, and compatibility expectations
- Verify tests fail before implementing
