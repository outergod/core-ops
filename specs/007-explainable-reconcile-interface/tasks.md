---
description: "Task list for explainable reconciliation interface"
---

# Tasks: Explainable Reconciliation Interface

**Input**: Design documents from `/specs/007-explainable-reconcile-interface/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are REQUIRED for this feature.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g. `US1`, `US2`, `US3`)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare contract fixtures, test scaffolding, and documentation targets for the new reconciliation output model

- [ ] T001 Create feature contract and scenario scaffolding references in `specs/007-explainable-reconcile-interface/contracts/reconciliation-output.md`
- [ ] T002 [P] Add explainable-interface integration test scaffolding in `tests/integration/test_status_contract.rs`
- [ ] T003 [P] Add plan/apply/result parity integration test scaffolding in `tests/integration/test_apply_report.rs`
- [ ] T004 [P] Add deterministic plan rendering regression scaffolding in `tests/integration/test_deterministic_planning.rs`
- [ ] T005 Register any new or expanded integration test coverage in `tests/integration/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define the shared public output model and transformation boundaries required by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T006 Define operator-facing machine-readable view types and enums in `src/core/types.rs`
- [ ] T007 [P] Add canonical managed-object identity builders and mapping helpers in `src/core/planner.rs`
- [ ] T008 [P] Add public-view construction helpers for causes, dependency edges, and semantic diffs in `src/cli/report.rs`
- [ ] T009 Add reconciliation output validation helpers for stable ordering and required fields in `src/core/validation.rs`
- [ ] T010 Preserve existing persisted provenance and deterministic state schema behavior while wiring new output-model inputs in `src/io/state.rs`
- [ ] T011 [P] Add unit tests for new public output types and ordering invariants in `tests/unit/test_types.rs`
- [ ] T012 [P] Add unit tests for canonical object identity and public-output validation rules in `tests/unit/test_validation.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Readable Reconciliation Plan (Priority: P1) 🎯 MVP

**Goal**: Deliver a deterministic, readable plan view whose machine-readable contract is authoritative and whose default presentation foregrounds changed objects while preserving full-scope coverage

**Independent Test**: Generate plans for representative desired-state revisions and verify that changed objects appear with stable identity, deterministic ordering, structured causes, prerequisite-oriented dependency context, and discoverable unchanged-object coverage without overwhelming the default view.

### Tests for User Story 1 ⚠️

- [ ] T013 [P] [US1] Add contract test for `PlanOutput` shape and field stability in `tests/integration/test_status_contract.rs`
- [ ] T014 [P] [US1] Add integration test for changed-first plan grouping and unchanged-object summary behavior in `tests/integration/test_deterministic_planning.rs`
- [ ] T015 [P] [US1] Add integration test for prerequisite, dependent, and blocker dependency inspection with deterministic order in `tests/integration/test_plan.rs`
- [ ] T016 [P] [US1] Add unit tests for plan-entry cause, diff derivation, and direct-versus-transitive dependency relations in `tests/unit/test_planner.rs`

### Implementation for User Story 1

- [ ] T017 [US1] Refactor deterministic planning output to build full-scope plan entries in `src/core/planner.rs`
- [ ] T018 [US1] Replace the legacy plan JSON renderer with the new `PlanOutput` contract in `src/cli/report.rs`
- [ ] T019 [US1] Update plan command output wiring to emit the new machine-readable plan payload in `src/cli/plan.rs`
- [ ] T020 [US1] Update human-readable plan rendering to be a deterministic projection of the new plan model in `src/cli/report.rs`
- [ ] T021 [US1] Update plan/status summaries to reflect changed, unchanged, blocked, and skipped plan semantics in `src/cli/status.rs`
- [ ] T022 [US1] Supersede the legacy structured diff contract with the new plan contract in `specs/006-deterministic-reconcile/contracts/structured-diff.md`

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Live Apply Visibility (Priority: P2)

**Goal**: Expose deterministic, phase-aware apply progress as authoritative machine-readable apply events and matching human-readable output

**Independent Test**: Run apply against representative revisions and verify that phase transitions, object progress states, failure/blockage reporting, and plan-order narration appear consistently in both machine-readable and human-readable output.

### Tests for User Story 2 ⚠️

- [ ] T023 [P] [US2] Add contract test for `ApplyOutput` phase, execution event shape, and absent-versus-null optional field semantics in `tests/integration/test_status_contract.rs`
- [ ] T024 [P] [US2] Add integration test for deterministic phase/event narration during apply in `tests/integration/test_apply_report.rs`
- [ ] T025 [P] [US2] Add integration test for failure, blocked, and skipped apply reporting in `tests/integration/test_reconcile_apply.rs`
- [ ] T026 [P] [US2] Add unit tests for apply-event sequencing and plan-order grouping in `tests/unit/test_verification.rs`

### Implementation for User Story 2

- [ ] T027 [US2] Add operator-facing apply phase and execution event builders in `src/cli/report.rs`
- [ ] T028 [US2] Update apply orchestration to collect and emit `ApplyOutput` data in `src/cli/apply.rs`
- [ ] T029 [US2] Thread plan-consistent object identity and action semantics through apply reporting in `src/core/reconcile.rs`
- [ ] T030 [US2] Update human-readable apply rendering to derive from the authoritative apply model in `src/cli/report.rs`
- [ ] T031 [US2] Surface phase-aware apply summaries and failure impact details in `src/cli/status.rs`

**Checkpoint**: User Story 2 should be fully functional and testable independently

---

## Phase 5: User Story 3 - Explainable Results for Automation and Review (Priority: P3)

**Goal**: Deliver authoritative machine-readable result and explain views with stable identity, convergence classification, and human/machine semantic parity for automation and operator inspection

**Independent Test**: Compare human-visible plan/apply/result/explain output with machine-readable result and explain payloads and verify exact agreement on object identity, action meaning, dependency relationships, and convergence classification.

### Tests for User Story 3 ⚠️

- [ ] T032 [P] [US3] Add contract test for full-scope `ResultOutput` and `ExplainOutput` shapes in `tests/integration/test_status_contract.rs`
- [ ] T033 [P] [US3] Add integration test for result-view continuity across plan, apply, and final outcome in `tests/integration/test_apply_report.rs`
- [ ] T034 [P] [US3] Add integration test for explain output and single-object inspection behavior in `tests/integration/test_reconcile_provenance.rs`
- [ ] T035 [P] [US3] Add unit tests for convergence outcome mapping and explain-view derivation in `tests/unit/test_invariants.rs`

### Implementation for User Story 3

- [ ] T036 [US3] Add full-scope result and explain view builders from convergence, verification, and plan data in `src/cli/report.rs`
- [ ] T037 [US3] Replace legacy convergence JSON emission with the new `ResultOutput` contract in `src/cli/report.rs`
- [ ] T038 [US3] Update apply/report orchestration to emit result-view data after terminal completion in `src/cli/apply.rs`
- [ ] T039 [US3] Add single-object explain rendering and object-selection helpers in `src/cli/report.rs`
- [ ] T040 [US3] Preserve revision context and outcome continuity across plan/apply/result surfaces in `src/core/reconcile.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finalize docs, compatibility review, quickstart validation, and full-suite regression coverage

- [ ] T041 [P] Update developer and operator guidance for the new reconciliation output contract in `docs/development.md`
- [ ] T042 [P] Update CLI help text and output examples for plan/apply/report surfaces in `src/cli/args.rs`
- [ ] T043 [P] Validate quickstart scenarios for the new output model in `tests/integration/test_quickstart_validation.rs`
- [ ] T044 [P] Replace remaining legacy plan-schema assertions in `tests/integration/test_deterministic_planning.rs`
- [ ] T045 [P] Replace remaining legacy apply/result schema assertions in `tests/integration/test_apply_report.rs`
- [ ] T046 [P] Record release-version-policy review outcome for the in-place schema replacement in `specs/007-explainable-reconcile-interface/plan.md`
- [ ] T047 [P] Evaluate and apply the required package version update for the public contract change in `Cargo.toml`
- [ ] T048 [P] Add contract tests for enum stability and deterministic array ordering in `tests/integration/test_status_contract.rs`
- [ ] T049 [P] Validate representative plan/result rendering time against the 1-second interactive budget in `tests/integration/test_performance.rs`
- [ ] T050 Run the full reconciliation interface test workflow documented in `specs/007-explainable-reconcile-interface/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational completion - MVP slice
- **User Story 2 (Phase 4)**: Depends on User Story 1 because apply output must reuse the new authoritative plan model and identity semantics
- **User Story 3 (Phase 5)**: Depends on User Story 1 and User Story 2 because result/explain continuity relies on the plan and apply models already being authoritative
- **Polish (Phase 6)**: Depends on all implemented user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: First deliverable after Foundational; no dependency on other stories
- **User Story 2 (P2)**: Requires User Story 1 public plan model and contract replacement
- **User Story 3 (P3)**: Requires User Story 1 plan identity/causes plus User Story 2 apply-event semantics for full continuity

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Public contract types and builders before renderer replacement
- Machine-readable output before human-readable rendering changes
- Continuity and summary/status wiring after the authoritative view model is in place
- Story checkpoint must pass before advancing to the next dependent story

### Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001
- T007, T008, T011, and T012 can run in parallel once T006 defines the shared public model
- T013, T014, T015, and T016 can run in parallel for US1
- T023, T024, T025, and T026 can run in parallel for US2
- T032, T033, T034, and T035 can run in parallel for US3
- T041, T042, T043, T044, T045, T046, T047, T048, and T049 can run in parallel in Polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 tests together:
Task: "Add contract test for PlanOutput shape and field stability in tests/integration/test_status_contract.rs"
Task: "Add integration test for changed-first plan grouping and unchanged-object summary behavior in tests/integration/test_deterministic_planning.rs"
Task: "Add integration test for prerequisite-oriented dependency rendering and deterministic order in tests/integration/test_plan.rs"
Task: "Add unit tests for plan-entry cause and diff derivation in tests/unit/test_planner.rs"
```

## Parallel Example: User Story 2

```bash
# Launch User Story 2 tests together:
Task: "Add contract test for ApplyOutput phase and execution event shape in tests/integration/test_status_contract.rs"
Task: "Add integration test for deterministic phase/event narration during apply in tests/integration/test_apply_report.rs"
Task: "Add integration test for failure, blocked, and skipped apply reporting in tests/integration/test_reconcile_apply.rs"
Task: "Add unit tests for apply-event sequencing and plan-order grouping in tests/unit/test_verification.rs"
```

## Parallel Example: User Story 3

```bash
# Launch User Story 3 tests together:
Task: "Add contract test for ResultOutput and ExplainOutput shapes in tests/integration/test_status_contract.rs"
Task: "Add integration test for result-view continuity across plan, apply, and final outcome in tests/integration/test_apply_report.rs"
Task: "Add integration test for explain output and single-object inspection behavior in tests/integration/test_reconcile_provenance.rs"
Task: "Add unit tests for convergence outcome mapping and explain-view derivation in tests/unit/test_invariants.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Confirm the new `PlanOutput` contract, changed-first rendering, dependency explanation, and unchanged-object access behavior
5. Demo the plan-only MVP before moving to apply/result/explain flows

### Incremental Delivery

1. Complete Setup + Foundational -> public output model ready
2. Add User Story 1 -> Test independently -> Demo MVP
3. Add User Story 2 -> Test independently -> Demo phase-aware apply visibility
4. Add User Story 3 -> Test independently -> Demo result/explain parity for automation
5. Finish Polish and full-suite validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 plan model and contract replacement
   - Developer B: User Story 2 apply-event model and rendering
   - Developer C: User Story 3 result/explain model and continuity
3. Merge when each story reaches its independent checkpoint

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps each task to a specific user story for traceability
- Each user story is independently testable from CLI-visible and machine-readable behavior
- Tests are mandatory for this feature
- Include provenance/version/status assertions wherever revision context or outcome semantics change
- Include release-version-policy updates because this feature changes externally observable machine-readable schema and rendering behavior
- Verify tests fail before implementing
