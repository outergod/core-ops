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

- [X] T001 Create feature contract and scenario scaffolding references in `specs/007-explainable-reconcile-interface/contracts/reconciliation-output.md`
- [X] T002 [P] Add explainable-interface integration test scaffolding in `tests/integration/test_status_contract.rs`
- [X] T003 [P] Add plan/apply/result parity integration test scaffolding in `tests/integration/test_apply_report.rs`
- [X] T004 [P] Add deterministic plan rendering regression scaffolding in `tests/integration/test_deterministic_planning.rs`
- [X] T005 Register any new or expanded integration test coverage in `tests/integration/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define the shared public output model and transformation boundaries required by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T006 Define operator-facing machine-readable view types and enums in `src/core/types.rs`, including current and prior desired-state selector context (`requested_repository`, `requested_ref`, `last_applied_requested_repository`, `last_applied_requested_ref`) alongside immutable revision identity (`target_revision`, `last_applied_revision`) in public revision context
- [X] T007 [P] Add canonical managed-object identity builders and mapping helpers in `src/core/planner.rs`
- [X] T008 [P] Add public-view construction helpers for causes, dependency edges, and semantic diffs in `src/cli/report.rs`
- [X] T009 Add reconciliation output validation helpers for stable ordering and required fields in `src/core/validation.rs`
- [X] T010 Preserve existing persisted provenance and deterministic state schema behavior while wiring new output-model inputs in `src/io/state.rs`
- [X] T011 [P] Add unit tests for new public output types and ordering invariants in `tests/unit/test_types.rs`
- [X] T012 [P] Add unit tests for canonical object identity and public-output validation rules in `tests/unit/test_validation.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Readable Reconciliation Plan (Priority: P1) 🎯 MVP

**Goal**: Deliver a deterministic, readable plan view whose machine-readable contract is authoritative and whose default presentation foregrounds changed or recovery-relevant objects while preserving full-scope coverage

**Independent Test**: Generate plans for representative desired-state revisions and verify that changed or recovery-relevant objects appear with stable identity, deterministic ordering, structured causes, and explanatory prerequisite-oriented dependency context, while unchanged objects remain discoverable without overwhelming the default view and unchanged dependency trees stay collapsed by default unless needed for explanation.

### Tests for User Story 1 ⚠️

- [X] T013 [P] [US1] Add contract test for `PlanOutput` shape and field stability in `tests/integration/test_status_contract.rs`
- [X] T014 [P] [US1] Add integration test for changed-or-recovery-first plan grouping, unchanged-object summary behavior, and default collapse of unchanged dependency trees in `tests/integration/test_deterministic_planning.rs`
- [X] T015 [P] [US1] Add integration test for prerequisite, dependent, and blocker dependency inspection with deterministic order in `tests/integration/test_plan.rs`
- [X] T016 [P] [US1] Add unit tests for plan-entry cause, diff derivation, and direct-versus-transitive dependency relations in `tests/unit/test_planner.rs`
- [X] T016a [P] [US1] Add tests for `update` versus `restart` versus `recover` action semantics and readable `object [action]` / `because` rendering in `tests/integration/test_deterministic_planning.rs`

### Implementation for User Story 1

- [X] T017 [US1] Refactor deterministic planning output to build full-scope plan entries in `src/core/planner.rs`
- [X] T018 [US1] Replace the legacy plan JSON renderer with the new `PlanOutput` contract in `src/cli/report.rs`
- [X] T019 [US1] Update plan command output wiring to emit the new machine-readable plan payload in `src/cli/plan.rs`
- [X] T019a [US1] Add `core-ops plan --json` support so the authoritative `PlanOutput` payload is user-selectable from `src/cli/args.rs`, `src/main.rs`, and `src/cli/plan.rs`
- [X] T020 [US1] Update human-readable plan rendering to be a deterministic projection of the new plan model, including recovery-oriented plan actions, selective dependency expansion only when explanatory, and secondary requested-ref context beside the immutable target revision when meaningful, in `src/cli/report.rs`
- [X] T021 [US1] Update plan/status summaries to reflect changed, recover, unchanged, blocked, and skipped plan semantics while keeping unchanged prerequisite trees collapsed by default and revision context rendering aligned with immutable-primary requested-ref-secondary behavior in `src/cli/status.rs`
- [X] T022 [US1] Supersede the legacy structured diff contract with the new plan contract in `specs/006-deterministic-reconcile/contracts/structured-diff.md`
- [X] T022a [US1] Refine deterministic plan action derivation so dependency-driven runtime reactivation is classified as `restart`, direct object definition changes remain `update`, and runtime non-convergence with unchanged declarative state is classified as `recover` with runtime-variance causes in `src/core/planner.rs`

**Checkpoint**: User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Live Apply Visibility (Priority: P2)

**Goal**: Deliver humane live apply output for operators while keeping authoritative machine-readable apply events as the underlying source of truth

**Independent Test**: Run apply against representative revisions and verify that default human output foregrounds acted-on, failed, and blocked objects without raw JSON or debug dumps, verbose mode adds phases and diagnostics as supporting detail, structured mode emits only machine-readable events, and unchanged objects are not misreported as skipped.

### Tests for User Story 2 ⚠️

- [X] T023 [P] [US2] Add contract test for `ApplyOutput` mode separation, event shape, `recover`/`recovered` semantics, and absent-versus-null optional field semantics in `tests/integration/test_status_contract.rs`
- [X] T024 [P] [US2] Add integration test for concise default human apply narration versus verbose phase-aware rendering in `tests/integration/test_apply_report.rs`
- [X] T024a [P] [US2] Add integration test for first-run versus recovery apply/result labeling in `tests/integration/test_apply_report.rs`
- [X] T025 [P] [US2] Add integration test for failure, blocked, and true-skipped apply reporting, explicitly distinguishing unchanged/no-op objects from skipped ones, in `tests/integration/test_reconcile_apply.rs`
- [X] T026 [P] [US2] Add unit tests for apply-event sequencing, plan-order grouping, and stable user-facing terminal state vocabulary including `recovered` in `tests/unit/test_verification.rs`

### Implementation for User Story 2

- [X] T027 [US2] Add operator-facing apply event builders with explicit default, verbose, and structured rendering modes, including `recover` actions and `recovered` outcomes, in `src/cli/report.rs`
- [X] T028 [US2] Update apply orchestration to collect `ApplyOutput` data and emit structured-only output for machine mode, preserving recovery-oriented event semantics, in `src/cli/apply.rs`
- [X] T029 [US2] Thread plan-consistent object identity, action semantics, recovery intent, and unchanged-versus-skipped distinctions through apply reporting in `src/core/reconcile.rs`
- [X] T030 [US2] Update human-readable apply rendering to derive from the authoritative apply model while suppressing raw JSON/provenance dumps, default hidden phases, surfacing recovery actions, and rendering meaningful current requested refs secondarily beside immutable target revisions plus meaningful prior requested refs secondarily beside prior immutable revisions in human headers and context, in `src/cli/report.rs`
- [X] T031 [US2] Surface humane apply summaries, failure impact details, and verbose-only phase visibility in `src/cli/status.rs`
- [X] T031a [US2] Add provenance-aware first-run, recovery, and managed-transition header rendering in `src/cli/report.rs` and `src/cli/status.rs`, including prior requested-ref context beside prior immutable revisions when available

**Checkpoint**: User Story 2 should be fully functional and testable independently

---

## Phase 5: User Story 3 - Explainable Results for Automation and Review (Priority: P3)

**Goal**: Deliver authoritative machine-readable result and explain views with stable identity, convergence classification, and human/machine semantic parity for automation and operator inspection

**Independent Test**: Compare human-visible plan/apply/result/explain output with machine-readable result and explain payloads and verify exact agreement on object identity, action meaning, dependency relationships, and convergence classification.

### Tests for User Story 3 ⚠️

- [X] T032 [P] [US3] Add contract test for full-scope `ResultOutput` and `ExplainOutput` shapes in `tests/integration/test_status_contract.rs`, including preserved current and prior selector context (`requested_repository` / `requested_ref`, `last_applied_requested_repository` / `last_applied_requested_ref`) without replacing immutable current or prior revision identity
- [X] T033 [P] [US3] Add integration test for result-view continuity across plan, apply, and final outcome in `tests/integration/test_apply_report.rs`
- [X] T033a [P] [US3] Add integration test for replaying structured apply events into humane output in `tests/integration/test_apply_report.rs`
- [X] T034 [P] [US3] Add integration test for explain output and single-object inspection behavior, including mount and automount metadata blocks, in `tests/integration/test_reconcile_provenance.rs`
- [X] T035 [P] [US3] Add unit tests for convergence outcome mapping and explain-view derivation in `tests/unit/test_invariants.rs`

### Implementation for User Story 3

- [X] T036 [US3] Add full-scope result and explain view builders from convergence, verification, and plan data, with object-specific metadata available to explain views, in `src/cli/report.rs`
- [X] T036a [US3] Add structured event replay and humane rendering helpers for transported apply events in `src/cli/report.rs`
- [X] T037 [US3] Replace legacy convergence JSON emission with the new `ResultOutput` contract in `src/cli/report.rs`
- [X] T038 [US3] Update apply/report orchestration to emit result-view data after terminal completion in `src/cli/apply.rs`
- [X] T039 [US3] Add single-object explain rendering and object-selection helpers, including mount and automount metadata sections plus humane rendering of current requested-ref context beside immutable target revision and prior requested-ref context in `Last:`-style output beside prior immutable revisions in explain and other targeted human-readable views, in `src/cli/report.rs`
- [X] T040 [US3] Preserve revision context and outcome continuity across plan/apply/result/explain surfaces in `src/core/reconcile.rs`, keeping reconciliation and rollback semantics anchored to immutable revisions, preserving `requested_repository` / `requested_ref` plus `last_applied_requested_repository` / `last_applied_requested_ref`, and ensuring human-readable views keep immutable short revisions primary while rendering meaningful current and prior requested refs secondarily

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finalize docs, compatibility review, quickstart validation, and full-suite regression coverage

- [X] T041 [P] Update developer and operator guidance for the new reconciliation output contract in `docs/development.md`
- [X] T042 [P] Update CLI help text and output examples for plan/apply/report surfaces in `src/cli/args.rs`
- [X] T043 [P] Validate quickstart scenarios for the new output model in `tests/integration/test_quickstart_validation.rs`
- [X] T044 [P] Replace remaining legacy plan-schema assertions in `tests/integration/test_deterministic_planning.rs`
- [X] T045 [P] Replace remaining legacy apply/result schema assertions in `tests/integration/test_apply_report.rs`
- [X] T046 [P] Record release-version-policy review outcome for the in-place schema replacement in `specs/007-explainable-reconcile-interface/plan.md`
- [X] T047 [P] Evaluate and apply the required package version update for the public contract change in `Cargo.toml`
- [X] T048 [P] Add contract tests for enum stability and deterministic array ordering in `tests/integration/test_status_contract.rs`
- [X] T049 [P] Validate representative plan/result rendering time against the 1-second interactive budget in `tests/integration/test_performance.rs`
- [X] T050 Run the full reconciliation interface test workflow documented in `specs/007-explainable-reconcile-interface/quickstart.md`

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
- **User Story 2 (P2)**: Requires User Story 1 public plan model and contract replacement, including recovery-oriented plan identity and action semantics
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
- T013, T014, T015, T016, and T016a can run in parallel for US1
- T023, T024, T025, and T026 can run in parallel for US2
- T032, T033, T034, and T035 can run in parallel for US3
- T041, T042, T043, T044, T045, T046, T047, T048, and T049 can run in parallel in Polish

---

## Parallel Example: User Story 1

```bash
# Launch User Story 1 tests together:
Task: "Add contract test for PlanOutput shape and field stability in tests/integration/test_status_contract.rs"
Task: "Add integration test for changed-or-recovery-first plan grouping, unchanged-object summary behavior, and default collapse of unchanged dependency trees in tests/integration/test_deterministic_planning.rs"
Task: "Add integration test for prerequisite-oriented dependency rendering and deterministic order, including selective explanatory expansion, in tests/integration/test_plan.rs"
Task: "Add tests for recover action semantics, grouping, and runtime-variance rendering in tests/integration/test_deterministic_planning.rs"
```

## Parallel Example: User Story 2

```bash
# Launch User Story 2 tests together:
Task: "Add contract test for ApplyOutput phase, execution event shape, and recover/recovered semantics in tests/integration/test_status_contract.rs"
Task: "Add integration test for humane live apply narration with verbose-only phase detail in tests/integration/test_apply_report.rs"
Task: "Add integration test for failure, blocked, and skipped apply reporting in tests/integration/test_reconcile_apply.rs"
Task: "Add unit tests for apply-event sequencing, plan-order grouping, and recovery terminal vocabulary in tests/unit/test_verification.rs"
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
4. **STOP and VALIDATE**: Confirm the new `PlanOutput` contract, changed-or-recovery-first rendering, selective dependency explanation, recovery intent semantics, and unchanged-object access behavior
5. Demo the plan-only MVP before moving to apply/result/explain flows

### Incremental Delivery

1. Complete Setup + Foundational -> public output model ready
2. Add User Story 1 -> Test independently -> Demo MVP
3. Add User Story 2 -> Test independently -> Demo humane live apply visibility with recovery-aware events
4. Add User Story 3 -> Test independently -> Demo result/explain parity for automation
5. Finish Polish and full-suite validation

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 plan model, recovery semantics, and contract replacement
   - Developer B: User Story 2 apply-event model, recovery vocabulary, and rendering
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
