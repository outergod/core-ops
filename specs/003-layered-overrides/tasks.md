---

description: "Task list for layered overrides feature"
---

# Tasks: Layered Overrides for Reusable Desired State

**Input**: Design documents from `/specs/003-layered-overrides/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and test fixture scaffolding

- [X] T001 Create layered overrides fixture repo in `tests/fixtures/layered_overrides/README.md`
- [X] T002 [P] Add fixture service/host files under `tests/fixtures/layered_overrides/services/` and `tests/fixtures/layered_overrides/hosts/`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core data structures, repo loading, and evaluation pipeline primitives

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Define layered override data structures in `src/core/types.rs`
- [X] T004 Implement host identity selection config (CLI/env) in `src/cli/args.rs`
- [X] T005 Wire host identity override into runtime config in `src/main.rs`
- [X] T006 Implement repository loader for `services/` and `hosts/<host>/host.yaml` in `src/io/repo.rs`
- [X] T007 Implement base/overlay drop-in discovery helpers in `src/io/repo.rs`
- [X] T008 Add validation rules for service selection + drop-in targets in `src/core/validation.rs`
- [X] T009 Add evaluation pipeline module scaffold in `src/core/evaluate.rs`
- [X] T010 Export evaluation module from `src/core/mod.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Reuse shared base definitions across hosts (Priority: P1) 🎯 MVP

**Goal**: Host-level service selection and base artifacts evaluation without overlays

**Independent Test**: Two hosts with different service selections produce different concrete desired states limited to their selections.

### Tests for User Story 1 ⚠️

- [X] T011 [P] [US1] Unit test for host selection parsing in `tests/unit/test_repo_selection.rs`
- [X] T012 [P] [US1] Integration test for host-specific service selection in `tests/integration/test_service_selection.rs`

### Implementation for User Story 1

- [X] T013 [US1] Load host declaration and selected services in `src/io/repo.rs`
- [X] T014 [US1] Build service catalog from `services/` in `src/io/repo.rs`
- [X] T015 [US1] Produce evaluated desired state from selected base artifacts in `src/core/evaluate.rs`
- [X] T016 [US1] Integrate evaluation output into planner inputs in `src/core/planner.rs`
- [X] T017 [US1] Update reconcile flow to use evaluated desired state in `src/core/reconcile.rs`

**Checkpoint**: User Story 1 functional and testable

---

## Phase 4: User Story 2 - Apply host-specific drop-ins without templating (Priority: P2)

**Goal**: Apply host-specific drop-ins after base drop-ins using native ordering rules

**Independent Test**: A host-specific drop-in changes only the target host’s concrete desired state.

### Tests for User Story 2 ⚠️

- [X] T018 [P] [US2] Unit test for drop-in ordering/precedence in `tests/unit/test_dropin_order.rs`
- [X] T019 [P] [US2] Integration test for host overlays application in `tests/integration/test_host_overrides.rs`
- [X] T041 [P] [US2] Integration test for stale managed config removal via root scanning in `tests/integration/test_config_cleanup.rs`
- [X] T042 [P] [US2] Integration test for managed config root closure behavior, including service variants whose names differ from their target roots, in `tests/integration/test_config_roots.rs`

### Implementation for User Story 2

- [X] T020 [US2] Apply base drop-ins in lexicographic order in `src/core/evaluate.rs`
- [X] T021 [US2] Apply host overrides after base drop-ins in `src/core/evaluate.rs`
- [X] T022 [US2] Validate drop-in targets and file types in `src/core/validation.rs`
- [X] T023 [US2] Track applied source layers in evaluated artifacts in `src/core/types.rs`


- [X] T035 [US2] Define config payload model (base + host overrides) in `src/core/types.rs`
- [X] T036 [US2] Load config payloads from `services/<service>/config/` in `src/io/repo.rs`
- [X] T037 [US2] Load host config overrides from `hosts/<host>/overrides/config/` in `src/io/repo.rs`
- [X] T038 [US2] Apply config layering rules (whole-file replacement + directory overlay) in `src/core/evaluate.rs`
- [X] T039 [US2] Materialize config payloads to bounded host paths in `src/io/apply.rs`
- [X] T040 [US2] Validate bounded config roots and reject unsupported formats in `src/core/validation.rs`
- [X] T043 [US2] Add managed config roots to desired state as boundaries derived from evaluated config payload target paths in `src/core/types.rs`
- [X] T044 [US2] Read managed config roots when building observed state in `src/io/observed.rs`

**Checkpoint**: User Story 2 functional and testable

---

## Phase 5: User Story 3 - Deterministic, testable evaluation (Priority: P3)

**Goal**: Deterministic evaluation with explicit failure behavior

**Independent Test**: Repeated evaluation yields identical outputs; invalid inputs fail before planning.

### Tests for User Story 3 ⚠️

- [X] T024 [P] [US3] Unit test for deterministic evaluation ordering in `tests/unit/test_evaluation_determinism.rs`
- [X] T025 [P] [US3] Integration test for invalid overlay failure in `tests/integration/test_overlay_validation.rs`

### Implementation for User Story 3

- [X] T026 [US3] Ensure stable ordering of artifacts/drop-ins in `src/core/evaluate.rs`
- [X] T027 [US3] Surface evaluation diagnostics in `src/core/errors.rs`
- [X] T028 [US3] Add evaluation audit output for plan/apply in `src/core/audit.rs`

**Checkpoint**: User Story 3 functional and testable

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, validation, and maintenance tasks

- [X] T029 [P] Add performance benchmark for evaluation overhead in `tests/integration/test_performance.rs`
- [X] T030 [P] Add test that rejects templating placeholders in configs in `tests/unit/test_no_templating.rs`
- [X] T031 [P] Add validation test for Quadlet vs systemd drop-in distinction in `tests/unit/test_dropin_types.rs`
- [X] T032 [P] Update quickstart validation to include layered override fixtures in `tests/integration/test_quickstart_validation.rs`
- [X] T033 [P] Update developer documentation for layered overrides in `docs/development.md`
- [X] T034 Run quickstart validation and integration tests referenced in `specs/003-layered-overrides/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2)
- **User Story 2 (P2)**: Can start after Foundational (Phase 2), builds on US1 evaluation output
- **User Story 3 (P3)**: Can start after Foundational (Phase 2), validates US1/US2 behavior

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Repository load/validation before evaluation logic
- Evaluation logic before planner/reconcile integration

### Parallel Opportunities

- T002 can run in parallel with T001
- Unit tests (T011, T018, T024) can run in parallel with their paired integration tests (T012, T019, T025)
- Documentation updates (T032, T033) can run in parallel

---

## Parallel Example: User Story 1

- Parallel set: T011 + T012 (tests), then T013 + T014, then T015, then T016 + T017
