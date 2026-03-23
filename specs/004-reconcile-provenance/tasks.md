---

description: "Task list for provenance and reconciliation revision tracking"
---

# Tasks: Provenance and Reconciliation Revision Tracking

**Input**: Design documents from `/specs/004-reconcile-provenance/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add provenance-specific fixtures and test scaffolding

- [X] T001 Create persisted provenance fixture snapshots in `tests/fixtures/provenance_state/`
- [X] T002 [P] Create valid/invalid canonical status snapshot fixtures in `tests/fixtures/provenance_state/`
- [X] T003 [P] Add fixture-driven status contract test scaffolding in `tests/integration/test_status_contract.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core provenance types, persisted-state boundary, and runtime configuration primitives

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Define persisted provenance state types and invariants in `src/core/types.rs`
- [X] T005 Add provenance/state-specific error variants in `src/core/errors.rs`
- [X] T006 Create canonical persisted provenance IO boundary in `src/io/state.rs`
- [X] T007 Export persisted provenance IO boundary from `src/io/mod.rs`
- [X] T008 Add CLI/runtime state path configuration for provenance status file in `src/cli/args.rs`
- [X] T009 Wire provenance state path configuration into command execution in `src/main.rs`
- [X] T010 [P] Add unit test coverage for provenance type invariants in `tests/unit/test_invariants.rs`
- [X] T011 [P] Add unit test coverage for persisted snapshot parsing/validation in `tests/unit/test_state_snapshot.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Explain Current Host State (Priority: P1) 🎯 MVP

**Goal**: Persist and expose canonical current-state provenance for a host through a status file and mirrored CLI surface

**Independent Test**: Trigger a successful reconciliation, restart CoreOps, and verify the canonical status file and CLI status output report the same controller provenance, desired-state provenance, applied revision, status, and generation.

### Tests for User Story 1 ⚠️

- [X] T012 [P] [US1] Unit test for atomic snapshot read/write behavior in `tests/unit/test_state_snapshot.rs`
- [X] T013 [P] [US1] Integration test for canonical status file persistence across restart in `tests/integration/test_reboot_recovery.rs`
- [X] T014 [P] [US1] Integration test for CLI status reflecting canonical file contents in `tests/integration/test_status_state.rs`

### Implementation for User Story 1

- [X] T015 [US1] Implement canonical provenance snapshot serialization and validation in `src/io/state.rs`
- [X] T016 [US1] Capture controller identity provenance for persisted snapshots in `src/main.rs`
- [X] T017 [US1] Persist successful current-state provenance snapshots from reconcile/apply flows in `src/cli/apply.rs`
- [X] T018 [US1] Persist successful current-state provenance snapshots from agent flows in `src/cli/agent.rs`
- [X] T019 [US1] Implement CLI status loading/rendering from the canonical status file in `src/cli/status.rs`
- [X] T020 [US1] Wire the status command to the canonical provenance file path in `src/main.rs`

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Distinguish Observed from Applied Revisions (Priority: P2)

**Goal**: Persist operational provenance that separates observed, attempted, and applied revisions across success, failure, in-progress, and never-run states

**Independent Test**: Cause CoreOps to observe a newer revision and fail reconciliation; verify the persisted snapshot and mirrored CLI output preserve the new attempted revision, the unchanged last applied revision, explicit status, explicit divergence, and monotonic generation.

### Tests for User Story 2 ⚠️

- [X] T021 [P] [US2] Unit test for reconciliation state transitions and generation monotonicity in `tests/unit/test_state_snapshot.rs`
- [X] T022 [P] [US2] Integration test for desired-state provenance fields and failed reconciliation preserving last applied revision in `tests/integration/test_reconcile_provenance.rs`
- [X] T023 [P] [US2] Integration test for explicit never-run and in-progress status representation in `tests/integration/test_status_state.rs`
- [X] T024 [P] [US2] Integration test for host-scoped desired-state provenance in `tests/integration/test_reconcile_provenance.rs`

### Implementation for User Story 2

- [X] T025 [US2] Implement reconciliation provenance transition rules in `src/core/reconcile.rs`
- [X] T026 [US2] Extend persisted snapshot model with desired-state provenance fields, attempted/applied divergence, status semantics, and generation rules in `src/core/types.rs`
- [X] T027 [US2] Write desired-state provenance plus in-progress, success, failed, and never-run snapshots in `src/io/state.rs`
- [X] T028 [US2] Integrate repository, requested ref, observed revision, and observed timestamp updates into apply lifecycle in `src/cli/apply.rs`
- [X] T029 [US2] Integrate repository, requested ref, observed revision, and observed timestamp updates into agent lifecycle in `src/cli/agent.rs`
- [X] T030 [US2] Ensure invalid, partial, or unsupported persisted snapshots are ignored as absent and can be rebuilt after fresh observation in `src/io/state.rs`

**Checkpoint**: User Story 2 should be fully functional and testable independently

---

## Phase 5: User Story 3 - Compare Runs Across Environments (Priority: P3)

**Goal**: Provide stable machine-readable provenance surfaces and contracts so operators can compare snapshots across runs and environments without relying on internal implementation details

**Independent Test**: Compare two canonical snapshots from different runs or hosts and determine whether the behavioral difference comes from controller identity, desired-state observation, or reconciliation outcome.

### Tests for User Story 3 ⚠️

- [X] T031 [P] [US3] Integration test for snapshot comparison scenarios in `tests/integration/test_status_contract.rs`
- [X] T032 [P] [US3] Integration test for CLI/status contract conformance in `tests/integration/test_status_state.rs`
- [X] T033 [P] [US3] Integration test for invalid or missing persisted provenance being treated as absent and rebuilt after fresh observation in `tests/integration/test_status_state.rs`

### Implementation for User Story 3

- [X] T034 [US3] Align CLI status output with the canonical status-file contract in `src/cli/status.rs`
- [X] T035 [US3] Mirror canonical provenance snapshot contents into operator-facing reports without independent state in `src/cli/report.rs`
- [X] T036 [US3] Emit audit/journald provenance fields consistent with canonical persisted state in `src/core/audit.rs`
- [X] T037 [US3] Update audit IO to log machine-readable provenance fields derived from canonical state in `src/io/audit.rs`
- [X] T038 [US3] Implement explicit schema migration or incompatibility handling policy in `src/io/state.rs`
- [X] T039 [US3] Source controller version provenance from the package version in `Cargo.toml` within `src/main.rs`
- [X] T040 [P] [US3] Integration test for controller version provenance matching `Cargo.toml` in `tests/integration/test_status_contract.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, compatibility, and end-to-end validation across all stories

- [X] T041 [P] Update developer guidance for canonical persisted provenance state in `docs/development.md`
- [X] T042 [P] Update CLI usage/help text for provenance status paths and behavior in `src/cli/args.rs`
- [X] T043 [P] Validate quickstart scenarios against persisted provenance behavior in `tests/integration/test_quickstart_validation.rs`
- [X] T044 [P] Add schema compatibility tests for persisted provenance versions in `tests/unit/test_state_snapshot.rs`
- [X] T045 [P] Add test asserting no journal/history artifact is required for provenance state in `tests/unit/test_state_snapshot.rs`
- [X] T046 [P] Document minor-or-major version review outcomes for incompatible persisted schema changes in `specs/004-reconcile-provenance/plan.md`
- [X] T047 [P] Evaluate and apply any required controller version update in `Cargo.toml` for merged observable or compatibility-affecting changes
- [X] T048 Run the full test suite covering provenance state, status CLI, and reconcile flows with `cargo test`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - MVP slice
- **User Story 2 (P2)**: Can start after Foundational (Phase 2), but builds on the persisted snapshot model from US1
- **User Story 3 (P3)**: Can start after Foundational (Phase 2), but is most valuable after US1 and US2 status semantics exist

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- State model and invariants before IO writes
- IO persistence before CLI/report mirroring
- Lifecycle wiring before cross-surface comparison work

### Parallel Opportunities

- T002 and T003 can run in parallel after T001
- T010 and T011 can run in parallel once foundational types are sketched
- T012, T013, and T014 can run in parallel for US1
- T021, T022, T023, and T024 can run in parallel for US2
- T031, T032, T033, and T040 can run in parallel for US3
- T041, T042, T043, T044, T045, T046, and T047 can run in parallel in Polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 test tasks together:
Task: "Unit test for atomic snapshot read/write behavior in tests/unit/test_state_snapshot.rs"
Task: "Integration test for canonical status file persistence across restart in tests/integration/test_reboot_recovery.rs"
Task: "Integration test for CLI status reflecting canonical file contents in tests/integration/test_status_state.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Verify canonical status-file persistence and CLI mirroring
5. Demo current-state provenance reporting

### Incremental Delivery

1. Complete Setup + Foundational → provenance foundation ready
2. Add User Story 1 → Test independently → Demo MVP
3. Add User Story 2 → Test independently → Demo observed/attempted/applied semantics
4. Add User Story 3 → Test independently → Demo cross-run comparison behavior
5. Finish Polish and full-suite validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 persistence + CLI status
   - Developer B: User Story 2 transition semantics + lifecycle wiring
   - Developer C: User Story 3 comparison/reporting surfaces
3. Merge after each story reaches its independent checkpoint

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to a specific user story for traceability
- Each user story is independently testable from the canonical persisted provenance state
- Tests are mandatory for this feature
- Include provenance/status assertions in every changed reconcile path
- Treat the canonical local status file as the only authoritative persisted provenance source for this iteration
