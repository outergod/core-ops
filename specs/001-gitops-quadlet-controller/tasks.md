---

description: "Task list template for feature implementation"
---

# Tasks: GitOps Quadlet Controller

**Input**: Design documents from `/specs/001-gitops-quadlet-controller/`
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
- **Web app**: `backend/src/`, `frontend/src/`
- **Mobile**: `api/src/`, `ios/src/` or `android/src/`
- Paths shown below assume single project - adjust based on plan.md structure

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create project structure per implementation plan in `src/` and `tests/`
- [X] T002 Initialize Rust project in `Cargo.toml` and `src/main.rs`
- [X] T003 [P] Configure formatting/linting in `rustfmt.toml` and `.clippy.toml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Define core domain types in `src/core/types.rs` (DesiredState, Workload, ObservedState, PlanAction, ReconciliationPlan, ReconcileRun, AuditRecord)
- [X] T005 Implement validation rules in `src/core/validation.rs` (Quadlet types, boundaries, invariants)
- [X] T006 Implement diffing model in `src/core/diff.rs` (desired vs observed, stable identifiers)
- [X] T007 Implement planning logic in `src/core/planner.rs` (validate → plan; idempotent action list)
- [X] T008 Define failure classes and error types in `src/core/errors.rs`
- [X] T009 Implement audit record structures in `src/core/audit.rs` (plan summary, actions, outcomes)
- [X] T010 [P] Add unit tests for domain types and invariants in `tests/unit/test_types.rs`
- [X] T011 [P] Add unit tests for validation rules in `tests/unit/test_validation.rs`
- [X] T012 [P] Add unit tests for diffing and planning logic in `tests/unit/test_planner.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Git-defined workload convergence (Priority: P1) 🎯 MVP

**Goal**: Converge host workloads to the Git-defined desired state within supported boundaries

**Independent Test**: Update the Git repository to add or change a workload and verify the host converges without manual steps

### Tests for User Story 1 (REQUIRED unless explicitly exempted) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T013 [P] [US1] Integration test for reconcile apply flow in `tests/integration/test_reconcile_apply.rs`
- [X] T014 [P] [US1] Integration test for desired state load from repo in `tests/integration/test_repo_load.rs`

### Implementation for User Story 1

- [X] T015 [P] [US1] Implement Git repo loader in `src/io/repo.rs` (clone/fetch + read Quadlet files) [requires URL support]
- [X] T016 [P] [US1] Implement Quadlet parser/loader in `src/io/quadlet.rs`
- [X] T017 [P] [US1] Implement observed state reader in `src/io/observed.rs` (systemd/Quadlet discovery)
- [X] T018 [US1] Implement apply adapter in `src/io/apply.rs` (write/remove Quadlet files, daemon-reload)
- [X] T019 [US1] Implement reconcile flow wiring in `src/core/reconcile.rs` (validate → plan → apply → verify)
- [X] T020 [US1] Wire CLI apply command in `src/cli/apply.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Plan and audit before apply (Priority: P2)

**Goal**: Provide a dry-run plan and audit trail for operator review

**Independent Test**: Run the controller in plan mode and verify it reports a clear plan without applying changes

### Tests for User Story 2 (REQUIRED unless explicitly exempted) ⚠️

- [X] T021 [P] [US2] Integration test for dry-run plan in `tests/integration/test_plan.rs`
- [X] T022 [P] [US2] Unit test for audit record formatting in `tests/unit/test_audit.rs`

### Implementation for User Story 2

- [X] T023 [US2] Implement plan-only execution path in `src/core/reconcile.rs` (skip apply)
- [X] T024 [US2] Implement plan output formatting in `src/cli/plan.rs`
- [X] T025 [US2] Implement audit record writer in `src/io/audit.rs` (filesystem storage)
- [X] T026 [US2] Wire CLI plan and status commands in `src/cli/plan.rs` and `src/cli/status.rs`

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: Audit Sink Default (US2 follow-up)

**Goal**: Default audit sink is the systemd journal; file export is explicit

**Independent Test**: Run plan and confirm structured audit events are emitted without requiring file persistence

- [X] T027 [US2] Implement structured audit sink to systemd journal, make file export optional, and fix CLI wiring in `src/io/audit.rs`, `src/cli/plan.rs`, and `src/main.rs`

---

## Phase 6: User Story 3 - Safe retry and failure handling (Priority: P3)

**Goal**: Provide explicit failure reporting and safe retries without compounding changes

**Independent Test**: Introduce an invalid configuration and verify the controller fails safely and reports clear errors

### Tests for User Story 3 (REQUIRED unless explicitly exempted) ⚠️

- [X] T028 [P] [US3] Integration test for validation failure handling in `tests/integration/test_validation_fail.rs`
- [X] T029 [P] [US3] Integration test for transient failure retry in `tests/integration/test_retry.rs`

### Implementation for User Story 3

- [X] T030 [US3] Implement failure classification and reporting in `src/core/errors.rs` and `src/cli/errors.rs`
- [X] T031 [US3] Implement retry policy in `src/core/retry.rs` (bounded, visible retries)
- [X] T032 [US3] Wire failure outputs to CLI in `src/cli/common.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 7: CLI Conventions & Diagnostics (MVP alignment)

**Goal**: Adopt clap + thiserror + miette for polished CLI parsing and errors

- [ ] T033 [P] Add CLI dependencies in `Cargo.toml` (clap derive, thiserror, miette)
- [ ] T034 [US2] Define clap-based CLI structs and subcommands in `src/cli/args.rs`
- [ ] T035 [US2] Map core errors to miette diagnostics in `src/cli/diagnostics.rs`
- [ ] T036 [US2] Wire `main.rs` to clap parsing and miette reporting

---

## Phase 8: Polish & Cross-Cutting Concerns
 
## Phase 4.5: Git URL Support (US1 follow-up)

**Goal**: Accept local paths or any valid Git URL (SSH/http(s)) as desired-state sources

- [X] T027A [US1] Add Git URL detection and clone/fetch logic in `src/io/repo.rs`
- [X] T027B [US1] Add tests for Git URL handling in `tests/integration/test_repo_load.rs`

---

## Phase 4.6: Repo File Filtering (US1 follow-up)

**Goal**: Ignore dotfiles and warn on unsupported extensions without failing

- [X] T027C [US1] Update quadlet loader to ignore dotfiles and warn on unsupported extensions in `src/io/quadlet.rs`
- [X] T027D [US1] Add tests for dotfile ignore and unknown extension warnings in `tests/integration/test_repo_load.rs`

**Purpose**: Improvements that affect multiple user stories

- [ ] T037 [P] Documentation updates in `docs/` (if created) or `README.md`
- [ ] T038 Code cleanup and refactoring in `src/`
- [ ] T039 [P] Additional unit tests for invariants in `tests/unit/test_invariants.rs`
- [ ] T040 Run quickstart validation against `specs/001-gitops-quadlet-controller/quickstart.md`

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
- **User Story 3 (P3)**: Can start after Foundational (Phase 2) - Uses shared error types but should be independently testable

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
Task: "Integration test for reconcile apply flow in tests/integration/test_reconcile_apply.rs"
Task: "Integration test for desired state load from repo in tests/integration/test_repo_load.rs"
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
