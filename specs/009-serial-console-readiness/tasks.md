# Tasks: Serial Console Readiness

**Input**: Design documents from `/specs/009-serial-console-readiness/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature. Rust changes MUST include the
standard validation gates `cargo test` and `cargo clippy --all-targets -- -D warnings`.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `tests/` at repository root
- Paths below follow the single-project Rust layout captured in `plan.md`

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the feature documentation and contract scaffolding for the readiness enhancement

- [X] T001 Create shared readiness fixture helpers and parser constants in /home/outergod/code/github.com/outergod/core-ops/src/core/verification_model.rs and /home/outergod/code/github.com/outergod/core-ops/tests/unit/test_verification_execution.rs
- [X] T002 [P] Add readiness contract fixture coverage in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_contracts.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish shared readiness data structures and boundary helpers that all user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T003 Add readiness record, readiness evidence, and guest handle support fields in /home/outergod/code/github.com/outergod/core-ops/src/core/verification_model.rs
- [X] T004 [P] Add pure readiness parsing, validation, and precedence helpers in /home/outergod/code/github.com/outergod/core-ops/src/core/verification_eval.rs
- [X] T005 [P] Extend libvirt boundary support for serial-console readiness acquisition and temporary fallback control in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T006 [P] Extend verification orchestration to call the new readiness acquisition boundary in /home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs
- [X] T007 [P] Add unit coverage for readiness parsing, IPv4 validation, and run-id/token matching in /home/outergod/code/github.com/outergod/core-ops/tests/unit/test_verification_execution.rs
- [X] T008 [P] Add contract coverage for the readiness record shape and machine-readable failure semantics in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_contracts.rs

**Checkpoint**: Shared readiness model and orchestration hooks are ready; user story implementation can now proceed

---

## Phase 3: User Story 1 - Reach Guests Reliably (Priority: P1) 🎯 MVP

**Goal**: Make serial-console guest self-report the primary VM-backed readiness and authoritative guest IPv4 path for healthy runs

**Independent Test**: Run a VM-backed verification scenario with a valid current-run readiness record and confirm the harness proceeds using the reported IPv4 address without depending primarily on ARP-derived discovery

### Tests for User Story 1 (REQUIRED) ⚠️

- [X] T009 [P] [US1] Add a unit test for first-valid-readiness acceptance and IPv4 selection in /home/outergod/code/github.com/outergod/core-ops/tests/unit/test_verification_execution.rs
- [X] T010 [P] [US1] Add an integration test for serial-console readiness taking precedence over ARP fallback in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_execution.rs

### Implementation for User Story 1

- [X] T011 [US1] Inject run-scoped readiness payload values into the VM-backed ignition rendering path in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T012 [US1] Add guest readiness service/script rendering support and serial-console marker handling in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T013 [US1] Implement host-side acceptance of the first valid readiness record and authoritative guest IPv4 selection in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T014 [US1] Thread the accepted readiness result into scenario execution and guest-boundary setup in /home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs
- [X] T015 [US1] Record accepted readiness evidence in retained verification artifacts in /home/outergod/code/github.com/outergod/core-ops/src/io/verification_artifacts.rs and /home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Reject Stale Or Wrong Readiness Records (Priority: P2)

**Goal**: Ensure stale, mismatched, and malformed readiness records cannot unblock the wrong run

**Independent Test**: Present stale-token, wrong-run, and malformed readiness records and confirm the harness ignores or rejects them until a valid current-run record appears

### Tests for User Story 2 (REQUIRED) ⚠️

- [X] T016 [P] [US2] Add unit tests for stale run-id/token rejection and malformed record rejection in /home/outergod/code/github.com/outergod/core-ops/tests/unit/test_verification_execution.rs
- [X] T017 [P] [US2] Add an integration test for console logs containing stale and malformed readiness records before a valid one in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_execution.rs
- [X] T018 [P] [US2] Add an integration test for previous-run serial console history replay in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_execution.rs

### Implementation for User Story 2

- [X] T019 [US2] Implement stale-record and malformed-record rejection summaries in pure readiness evaluation logic in /home/outergod/code/github.com/outergod/core-ops/src/core/verification_eval.rs
- [X] T020 [US2] Teach the libvirt readiness acquisition path to continue waiting after rejected console records and to preserve rejection evidence in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T021 [US2] Surface rejected readiness evidence and run-identity-safe diagnostics in /home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs and /home/outergod/code/github.com/outergod/core-ops/src/io/verification_artifacts.rs

**Checkpoint**: User Stories 1 and 2 should both work independently

---

## Phase 5: User Story 3 - Fail Explicitly When Readiness Never Arrives (Priority: P3)

**Goal**: End missing-readiness and malformed-readiness runs with explicit timeout or infrastructure-style outcomes that stay distinct from behavioral CoreOps failures

**Independent Test**: Run scenarios with no valid readiness record and with only malformed readiness records and confirm the harness reports explicit readiness-related failure outcomes within the configured readiness window

### Tests for User Story 3 (REQUIRED) ⚠️

- [X] T022 [P] [US3] Add unit tests for readiness timeout and readiness-failure outcome mapping in /home/outergod/code/github.com/outergod/core-ops/tests/unit/test_verification_results.rs
- [X] T023 [P] [US3] Add integration coverage for missing-readiness timeout and infrastructure-style readiness failure reporting in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_execution.rs
- [X] T024 [P] [US3] Add CLI/report coverage for readiness-related machine and humane output semantics in /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_cli.rs and /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_contracts.rs

### Implementation for User Story 3

- [X] T025 [US3] Implement bounded waiting and timeout classification for serial-console readiness acquisition in /home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs
- [X] T026 [US3] Distinguish readiness-related timeout and infrastructure-style failures from behavioral CoreOps failures in /home/outergod/code/github.com/outergod/core-ops/src/core/verification_eval.rs and /home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs
- [X] T027 [US3] Export readiness failure summaries and evidence references in /home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs and /home/outergod/code/github.com/outergod/core-ops/src/io/verification_artifacts.rs

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finish rollout controls, documentation, and full validation

- [X] T028 [P] Add or update developer documentation for serial-console readiness workflow in /home/outergod/code/github.com/outergod/core-ops/docs/development.md and /home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/quickstart.md
- [X] T029 [P] Add or update accepted verification scenario coverage expectations for readiness-related classes in /home/outergod/code/github.com/outergod/core-ops/docs/verification-behavior-inventory.md
- [X] T030 Verify provenance/version/status output remains consistent across changed verification flows and that humane and machine-readable readiness failure outputs stay aligned in /home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs and /home/outergod/code/github.com/outergod/core-ops/tests/integration/test_verification_contracts.rs
- [X] T031 Evaluate and document release-version-policy impact for the verification harness behavior change in /home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/plan.md and /home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/contracts/readiness-record-contract.md
- [X] T032 Run `cargo test` in /home/outergod/code/github.com/outergod/core-ops
- [X] T033 Run `cargo clippy --all-targets -- -D warnings` in /home/outergod/code/github.com/outergod/core-ops

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational - MVP slice
- **User Story 2 (P2)**: Depends on User Story 1’s readiness acceptance path being in place
- **User Story 3 (P3)**: Depends on the shared readiness acquisition path from User Story 1 and reuses rejection semantics from User Story 2

### Within Each User Story

- Tests must be written and fail before implementation
- Pure model/evaluation changes before boundary wiring where practical
- Boundary implementation before reporting/artifact integration
- Story complete before moving to the next priority

### Parallel Opportunities

- T002 can run in parallel with T001
- T004-T008 can run in parallel once T003 defines the shared model direction
- T009 and T010 can run in parallel
- T016-T018 can run in parallel
- T022-T024 can run in parallel
- T028 and T029 can run in parallel during polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 tests together:
Task: "Add a unit test for first-valid-readiness acceptance and IPv4 selection in tests/unit/test_verification_execution.rs"
Task: "Add an integration test for serial-console readiness taking precedence over ARP fallback in tests/integration/test_verification_execution.rs"

# Launch independent shared code work after the tests:
Task: "Inject run-scoped readiness payload values into the VM-backed ignition rendering path in src/io/libvirt.rs"
Task: "Add guest readiness service/script rendering support and serial-console marker handling in src/io/libvirt.rs"
```

## Parallel Example: User Story 2

```bash
# Launch User Story 2 tests together:
Task: "Add unit tests for stale run-id/token rejection and malformed record rejection in tests/unit/test_verification_execution.rs"
Task: "Add an integration test for console logs containing stale and malformed readiness records before a valid one in tests/integration/test_verification_execution.rs"
```

## Parallel Example: User Story 3

```bash
# Launch User Story 3 tests together:
Task: "Add unit tests for readiness timeout and readiness-failure outcome mapping in tests/unit/test_verification_results.rs"
Task: "Add integration coverage for missing-readiness timeout and infrastructure-style readiness failure reporting in tests/integration/test_verification_execution.rs"
Task: "Add CLI/report coverage for readiness-related machine and humane output semantics in tests/integration/test_verification_cli.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Run the User Story 1 tests and confirm a healthy
   VM-backed run can use serial-console readiness as the primary IPv4 path

### Incremental Delivery

1. Complete Setup + Foundational
2. Deliver User Story 1 for the primary happy path
3. Add User Story 2 for stale/malformed safety
4. Add User Story 3 for explicit timeout/failure semantics
5. Finish with documentation, policy review, and full validation

### Parallel Team Strategy

1. One developer can focus on pure readiness model/evaluation changes
2. One developer can focus on libvirt/ignition/console boundary changes
3. One developer can focus on integration and CLI/report coverage after the
   foundational model is in place

## Notes

- [P] tasks touch different files or can proceed after a common prerequisite
- Each user story remains independently testable
- Tests are mandatory for this feature
- Include provenance/version/status assertions where readiness behavior or run
  classification changes
- Keep ARP fallback temporary and subordinate in all implementation work
