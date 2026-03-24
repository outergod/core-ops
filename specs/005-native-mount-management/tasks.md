---
description: "Task list for native mount management"
---

# Tasks: Native Mount Management

**Input**: Design documents from `/specs/005-native-mount-management/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add mount-management fixtures, contracts scaffolding, and test entry points

- [X] T001 Create mount-management fixture directories and README in `tests/fixtures/mount_management/`
- [X] T002 [P] Add fixture scenarios for normal mounts, network-backed automounts, invalid definitions, and busy-removal cases in `tests/fixtures/mount_management/`
- [X] T003 [P] Add mount contract test scaffolding in `tests/integration/test_mount_contracts.rs`
- [X] T004 [P] Add mount reconciliation integration test scaffolding in `tests/integration/test_mount_reconcile.rs`
- [X] T005 Register new mount-management integration test modules in `tests/integration/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define shared mount data structures, validation, planning primitives, and native-unit generation boundaries

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Define mount declaration, mount dependency, prepared target path, generated unit set, and mount reconciliation result types in `src/core/types.rs`
- [X] T007 Add mount-specific validation and reconciliation error variants in `src/core/errors.rs`
- [X] T008 Implement mount declaration validation rules, ownership checks, and automount eligibility rules in `src/core/validation.rs`
- [X] T009 Implement desired-state repo loading for named mount declarations and service mount references in `src/io/repo.rs`
- [X] T010 Implement mount-aware evaluation output structures and dependency expansion in `src/core/evaluate.rs`
- [X] T011 Implement planner primitives for mount actions, dependency edits, and removal candidates in `src/core/planner.rs`
- [X] T012 Implement native mount and automount unit rendering plus service dependency materialization helpers in `src/core/unit.rs`
- [X] T013 Export mount-aware native unit generation through IO boundaries in `src/io/quadlet.rs`
- [X] T014 [P] Add unit tests for mount type invariants and identity rules in `tests/unit/test_types.rs`
- [X] T015 [P] Add unit tests for mount validation and ownership-boundary rules in `tests/unit/test_validation.rs`
- [X] T016 [P] Add unit tests for planner dependency expansion and deterministic unit generation in `tests/unit/test_planner.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Reconcile a Mount-Backed Service (Priority: P1) 🎯 MVP

**Goal**: Reconcile native mount units, bounded target-path preparation, and mount-backed service dependencies so a service can consume mounted storage without manual unit authoring

**Independent Test**: Declare an NFS-backed mount and a dependent service, run plan/apply, and verify the generated service unit contains the expected dependency semantics, the target path is prepared, the mount becomes active, and the service becomes runnable only after mount verification.

### Tests for User Story 1 ⚠️

- [X] T017 [P] [US1] Add integration test for planning mount units and generated service dependency semantics in `tests/integration/test_mount_contracts.rs`
- [X] T018 [P] [US1] Add integration test for apply creating prepared target paths and activating a required mount before service start in `tests/integration/test_mount_reconcile.rs`
- [X] T019 [P] [US1] Add unit tests for native mount and service dependency rendering in `tests/unit/test_verification.rs`

### Implementation for User Story 1

- [X] T020 [US1] Implement diff support for managed mount and automount artifacts in `src/core/diff.rs`
- [X] T021 [US1] Implement target-path preparation and bounded owner/group/mode handling in `src/io/apply.rs`
- [X] T022 [US1] Implement observed-state collection for native mount and automount units plus mounted target-path verification in `src/io/observed.rs`
- [X] T023 [US1] Implement mount verification rules combining native unit state and mounted path checks in `src/core/verify.rs`
- [X] T024 [US1] Implement reconciliation flow for normal mount activation and service unblocking in `src/core/reconcile.rs`
- [X] T025 [US1] Wire plan output to include mount actions, dependency edits, and prepared path actions in `src/cli/plan.rs`
- [X] T026 [US1] Wire apply flow to execute mount actions and prepared path actions through native systemd boundaries in `src/cli/apply.rs`
- [X] T027 [US1] Surface mount-backed service dependency results in status/report output in `src/cli/status.rs`

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Diagnose Mount Failures Explicitly (Priority: P2)

**Goal**: Expose explicit verification, degradation, recovery, and removal-failure behavior for managed mounts and dependent services

**Independent Test**: Reconcile an invalid or unreachable mount, verify the dependent service is blocked or degraded with explicit diagnostics, then recover the mount and confirm reconciliation converges; separately remove a managed mount and confirm busy removal fails explicitly.

### Tests for User Story 2 ⚠️

- [X] T028 [P] [US2] Add integration test for invalid mount declarations and blocked dependent services in `tests/integration/test_mount_failures.rs`
- [X] T029 [P] [US2] Add integration test for recovery after a previously failed mount becomes reachable in `tests/integration/test_mount_failures.rs`
- [X] T030 [P] [US2] Add integration test for managed mount removal and explicit busy-removal failure in `tests/integration/test_mount_removal.rs`
- [X] T031 [P] [US2] Add unit tests for removal-state transitions and degraded-service semantics in `tests/unit/test_evaluation_determinism.rs`

### Implementation for User Story 2

- [X] T032 [US2] Implement explicit mount failure, degraded dependency, and recovery transitions in `src/core/reconcile.rs`
- [X] T033 [US2] Implement diagnostics for validation, verification, and busy-removal failures in `src/cli/diagnostics.rs`
- [X] T034 [US2] Extend apply orchestration to stop dependent managed services before mount removal and fail on busy teardown in `src/io/apply.rs`
- [X] T035 [US2] Extend verification reporting to distinguish blocked, degraded, removing, removed, and busy states in `src/core/verify.rs`
- [X] T036 [US2] Emit mount failure and removal outcomes in machine-readable reports and audit output in `src/cli/report.rs`
- [X] T037 [US2] Include mount failure and removal details in journald audit payloads in `src/io/audit.rs`

**Checkpoint**: User Story 2 should be fully functional and testable independently

---

## Phase 5: User Story 3 - Reuse Mount-Aware Service Definitions Safely (Priority: P3)

**Goal**: Support reusable mount-aware service definitions, host-specific overrides within ownership boundaries, and network-backed automount behavior with correct native dependency materialization

**Independent Test**: Select reusable services with named mount declarations on two hosts, verify host overrides stay within declared ownership boundaries, and confirm an explicitly enabled NFS automount produces coherent path-based and explicit unit dependencies.

### Tests for User Story 3 ⚠️

- [X] T038 [P] [US3] Add integration test for reusable mount declarations with host-specific overrides in `tests/integration/test_mount_reuse.rs`
- [X] T039 [P] [US3] Add integration test for network-backed automount planning and generated native dependency semantics in `tests/integration/test_mount_contracts.rs`
- [X] T040 [P] [US3] Add integration test for automount apply and ordering behavior with dependent services in `tests/integration/test_mount_reconcile.rs`
- [X] T041 [P] [US3] Add unit tests for automount eligibility and explicit unit dependency generation in `tests/unit/test_verification.rs`

### Implementation for User Story 3

- [X] T042 [US3] Implement host-override merge rules for mount declarations and service mount references in `src/io/repo.rs`
- [X] T043 [US3] Implement planner support for automount artifacts and reuse-safe dependency generation in `src/core/planner.rs`
- [X] T044 [US3] Implement automount rendering and explicit unit dependency wiring for dependent services in `src/core/unit.rs`
- [X] T045 [US3] Extend observed-state collection to recognize automount lifecycle state and relationships to underlying mounts in `src/io/observed.rs`
- [X] T046 [US3] Extend reconcile/apply flow to activate network-backed automount declarations with correct native ordering semantics in `src/cli/apply.rs`
- [X] T047 [US3] Extend plan/status reporting to show reusable mount identities, override effects, and automount-specific dependency behavior in `src/cli/plan.rs`

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, provenance/versioning review, and end-to-end validation across all stories

- [X] T048 [P] Update operator and developer guidance for native mount declarations, automount limits, and removal behavior in `docs/development.md`
- [X] T049 [P] Update CLI help text for mount-aware plan/apply/status behavior in `src/cli/args.rs`
- [X] T050 [P] Validate mount quickstart scenarios in `tests/integration/test_quickstart_validation.rs`
- [X] T051 [P] Add regression coverage for mount-related config cleanup and idempotent reapply in `tests/integration/test_config_cleanup.rs`
- [X] T052 [P] Validate mount declaration and removal contract examples against implemented behavior in `tests/integration/test_mount_contracts.rs`
- [X] T053 [P] Record release-version-policy review outcome for mount behavior changes in `specs/005-native-mount-management/plan.md`
- [X] T054 [P] Evaluate and apply the required controller package-version update in `Cargo.toml`
- [X] T055 [P] [US2] Add integration test for mount-specific status output in `tests/integration/test_status_state.rs`
- [X] T056 [P] [US2] Add integration test for journald/audit payloads covering mount success, degradation, and busy removal in `tests/integration/test_journald_audit.rs`
- [X] T057 Run the full test suite for mount planning, apply, verification, ordering, cleanup, status, and audit behavior with `cargo test`

---

## Phase 7: Native-Artifact-Primary Redesign

**Purpose**: Replace the YAML-first mount source model with native `.mount` and optional `.automount` artifacts annotated by a bounded `[X-CoreOps]` section, and align implementation around that native-unit-primary design

Note: Phases 2-6 reflect the superseded YAML-first mount design and are retained only as historical context. Phase 7 defines the active implementation path for this feature.

- [X] T058 [P] Update feature documentation and contracts for native-artifact-primary mount metadata, native stem-derived references, and `CreateMountpoint` semantics in `specs/005-native-mount-management/spec.md`
- [X] T059 [P] Update implementation plan to describe minimal embedded `[X-CoreOps]` metadata, native-artifact-primary behavior, and reopened version review in `specs/005-native-mount-management/plan.md`
- [ ] T060 [P] Rework the `[X-CoreOps]` schema contract in `specs/005-native-mount-management/contracts/mount-declaration.md` and related spec references so managed `.mount` artifacts allow only `CreateMountpoint` (default `true`) and disallow `Id`, `PreparedPath`, ownership/mode fields, `RemovalPolicy`, and `VerificationMode`
- [ ] T061 [P] Add/update integration coverage in `tests/integration/test_mount_contracts.rs` for native `.mount`/`.automount` artifacts with minimal `[X-CoreOps]` metadata, including `CreateMountpoint` defaults, invalid removed fields, and native stem-derived service references
- [X] T062 [P] Add integration tests for native-artifact-primary mount parsing, systemd-like override layering, native stem-derived mount references, and service-defined consumer relationships in `tests/integration/test_mount_reuse.rs`
- [X] T063 Replace YAML-first mount declaration loading with parsing of embedded `[X-CoreOps]` metadata from user-authored native `.mount` and `.automount` artifacts while deriving managed mount references from native `.mount` stems and keeping service definitions authoritative for consumer relationships in `src/io/repo.rs`
- [ ] T064 Update validation rules in `src/core/validation.rs` and `tests/unit/test_verification.rs` so CoreOps accepts only the minimal `[X-CoreOps]` schema, derives service-referenced mount identity from the native `.mount` stem, rejects removed fields, and enforces `FR-013c` for service-referenced mounts
- [X] T065 Refactor evaluation and planner flows to operate on managed native mount artifacts and embedded metadata rather than YAML-first declarations in `src/core/evaluate.rs`
- [ ] T066 Simplify `[X-CoreOps]` parsing and normalization in `src/io/repo.rs` and related loaders to support only `CreateMountpoint` on managed `.mount` artifacts, apply systemd-like layering before validation, and remove parsing of `Id`, `PreparedPath`, ownership/mode, removal, and verification fields
- [ ] T067 Align apply and observed-state behavior in `src/io/apply.rs`, `src/io/observed.rs`, and related tests so `CreateMountpoint` controls mountpoint creation for `Where=`, retained `[X-CoreOps]` metadata does not cause drift unless effective CoreOps reconciliation semantics change, and removed metadata fields are ignored as unsupported input
- [ ] T068 Update operator diagnostics, reporting, and audit coverage in `src/cli/report.rs`, `src/cli/status.rs`, `src/io/audit.rs`, and related tests to use native `.mount` stem references, `CreateMountpoint` semantics, and error messages for unsupported `[X-CoreOps]` fields
- [X] T069 Record the redesigned release-version-policy review outcome for embedded `[X-CoreOps]` metadata in `specs/005-native-mount-management/plan.md`
- [ ] T070 Revalidate documentation, quickstart, and release readiness in `docs/development.md`, `specs/005-native-mount-management/quickstart.md`, and final validation notes so they describe the minimal `[X-CoreOps]` schema, native stem-derived references, and the simplified mountpoint-creation behavior

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
- **Polish (Phase 6)**: Depends on all desired user stories being complete
- **Native-Artifact-Primary Redesign (Phase 7)**: Uses Phase 1 setup as needed, supersedes Phases 2-6, and defines the authoritative implementation path that must complete before the feature is considered final

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - MVP slice
- **User Story 2 (P2)**: Can start after User Story 1 because it depends on working mount activation, verification, and reporting paths
- **User Story 3 (P3)**: Can start after User Story 1; it is most valuable after basic mount declarations and dependency materialization exist

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Type and validation changes before planner and unit generation changes
- Planner and generation changes before apply or observed-state wiring
- Verification and reporting changes before polishing and full-suite validation

### Parallel Opportunities

- T002, T003, and T004 can run in parallel after T001
- T014, T015, and T016 can run in parallel once foundational types are sketched
- T017, T018, and T019 can run in parallel for US1
- T028, T029, T030, and T031 can run in parallel for US2
- T038, T039, T040, and T041 can run in parallel for US3
- T048, T049, T050, T051, T052, T053, T054, T055, and T056 can run in parallel in Polish
- T058, T059, T060, T061, and T062 can run in parallel at the start of the redesign phase

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 tests together:
Task: "Add integration test for planning mount units and generated service dependency semantics in tests/integration/test_mount_contracts.rs"
Task: "Add integration test for apply creating prepared target paths and activating a required mount before service start in tests/integration/test_mount_reconcile.rs"
Task: "Add unit tests for native mount and service dependency rendering in tests/unit/test_verification.rs"
```

## Parallel Example: User Story 2

```bash
# Launch User Story 2 failure-path tests together:
Task: "Add integration test for invalid mount declarations and blocked dependent services in tests/integration/test_mount_failures.rs"
Task: "Add integration test for recovery after a previously failed mount becomes reachable in tests/integration/test_mount_failures.rs"
Task: "Add integration test for managed mount removal and explicit busy-removal failure in tests/integration/test_mount_removal.rs"
```

## Parallel Example: User Story 3

```bash
# Launch User Story 3 reuse and automount tests together:
Task: "Add integration test for reusable mount declarations with host-specific overrides in tests/integration/test_mount_reuse.rs"
Task: "Add integration test for network-backed automount planning and generated native dependency semantics in tests/integration/test_mount_contracts.rs"
Task: "Add unit tests for automount eligibility and explicit unit dependency generation in tests/unit/test_verification.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Verify native mount activation, target-path preparation, and generated service dependency semantics
5. Demo the mount-backed service workflow

### Incremental Delivery

1. Complete Setup + Foundational → mount foundation ready
2. Add User Story 1 → Test independently → Demo MVP
3. Add User Story 2 → Test independently → Demo failure, recovery, and removal behavior
4. Add User Story 3 → Test independently → Demo reusable declarations and network-backed automount support
5. Finish Polish and full-suite validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 mount activation, preparation, and dependency materialization
   - Developer B: User Story 2 failure, recovery, and removal semantics
   - Developer C: User Story 3 reusable declarations, overrides, and automount behavior
3. Merge after each story reaches its independent checkpoint

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to a specific user story for traceability
- Each user story is independently testable from plan/apply/status behavior
- Tests are mandatory for this feature
- Include provenance/status assertions where mount behavior, dependency semantics, or removal results change
- Include release-version-policy updates because this feature changes externally observable reconciliation behavior and generated native units
- Phase 7 supersedes the earlier YAML-first source-model assumption and must complete before this feature is considered settled
- Verify tests fail before implementing
