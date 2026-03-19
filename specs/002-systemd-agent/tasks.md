---

description: "Task list template for feature implementation"
---

# Tasks: Systemd-Managed Host Agent

**Input**: Design documents from `/specs/002-systemd-agent/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: The examples below include test tasks. Tests are REQUIRED unless the
feature spec explicitly documents a justified exemption.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths shown below assume single project

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [ ] T001 Add systemd unit templates for agent service and timer in `specs/002-systemd-agent/contracts/systemd/`
- [ ] T002 [P] Document agent deployment in `specs/002-systemd-agent/quickstart.md`
- [ ] T003 [P] Add agent configuration notes in `docs/development.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T004 Extend artifact types to include socket and volume in `src/core/types.rs`
- [ ] T005 Update Quadlet loader to parse socket and volume artifacts in `src/io/quadlet.rs`
- [ ] T006 Update diffing to handle mixed artifact types in `src/core/diff.rs`
- [ ] T007 Add ordering policy (Volume → Container → Socket) to planning in `src/core/planner.rs`
- [ ] T008 Add verification result model for artifact checks in `src/core/types.rs`
- [ ] T009 Define run lock interface and errors in `src/core/errors.rs` and `src/core/types.rs`
- [ ] T010 Implement run lock adapter in `src/io/lock.rs`
- [ ] T011 [P] Add unit tests for ordering rules in `tests/unit/test_planner.rs`
- [ ] T012 [P] Add unit tests for quadlet type parsing in `tests/unit/test_types.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Automated host agent runs unattended (Priority: P1) 🎯 MVP

**Goal**: Run reconciliation unattended via systemd service + timer

**Independent Test**: Enable the timer and verify a run occurs with journald output

### Tests for User Story 1 (REQUIRED unless explicitly exempted) ⚠️

- [ ] T013 [P] [US1] Integration test for single-run lock behavior in `tests/integration/test_agent_lock.rs`
- [ ] T014 [P] [US1] Integration test for systemd unit templates presence in `tests/integration/test_systemd_units.rs`

### Implementation for User Story 1

- [ ] T015 [US1] Implement agent run entrypoint (oneshot) in `src/cli/agent.rs`
- [ ] T016 [US1] Wire systemd timer/service invocation into CLI in `src/main.rs`
- [ ] T017 [US1] Implement run lock acquire/release in `src/io/lock.rs`
- [ ] T018 [US1] Emit journald audit events for agent runs in `src/io/audit.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Reconcile containers, sockets, and volumes (Priority: P2)

**Goal**: Support container, socket, and volume Quadlet artifacts with ordering

**Independent Test**: Reconcile a repo containing all three artifact types in one run

### Tests for User Story 2 (REQUIRED unless explicitly exempted) ⚠️

- [ ] T019 [P] [US2] Integration test for socket+volume reconciliation in `tests/integration/test_quadlet_artifacts.rs`
- [ ] T020 [P] [US2] Integration test for ordering (volume before container before socket) in `tests/integration/test_ordering.rs`

### Implementation for User Story 2

- [ ] T021 [US2] Extend observed state loading for socket and volume artifacts in `src/io/observed.rs`
- [ ] T022 [US2] Extend apply adapter for socket and volume artifacts in `src/io/apply.rs`
- [ ] T023 [US2] Update plan output to include artifact type in reports in `src/cli/report.rs`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Explicit verification and observability (Priority: P3)

**Goal**: Define verification behavior and journald observability for all artifacts

**Independent Test**: Introduce a failing artifact and confirm journald logs show failure details

### Tests for User Story 3 (REQUIRED unless explicitly exempted) ⚠️

- [ ] T024 [P] [US3] Integration test for verification failures in `tests/integration/test_verification.rs`
- [ ] T025 [P] [US3] Integration test for journald audit content in `tests/integration/test_journald_audit.rs`

### Implementation for User Story 3

- [ ] T026 [US3] Implement verification checks using systemd unit state in `src/core/verify.rs`
- [ ] T027 [US3] Wire verification into reconcile flow in `src/core/reconcile.rs`
- [ ] T028 [US3] Extend audit record with verification results in `src/core/audit.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T029 [P] Documentation updates in `specs/002-systemd-agent/quickstart.md`
- [ ] T030 Code cleanup and refactoring in `src/`
- [ ] T031 [P] Additional unit tests for artifact verification in `tests/unit/test_verification.rs`
- [ ] T032 Run quickstart.md validation against `specs/002-systemd-agent/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2 → P3)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P2)**: Can start after Foundational (Phase 2) - Integrates with US1 outputs but should be independently testable
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Uses shared verification outputs but should be independently testable

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Domain types/validation/diff/plan before apply adapters
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, all user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Integration test for single-run lock behavior in tests/integration/test_agent_lock.rs"
Task: "Integration test for systemd unit templates presence in tests/integration/test_systemd_units.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo
4. Add User Story 3 → Test independently → Deploy/Demo
5. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
   - Developer C: User Story 3
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Tests are mandatory unless explicitly exempted in the spec
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
