# Tasks: E2E Verification Harness with LLM-Assisted Scenario Generation

**Input**: Design documents from `/specs/008-e2e-verification-harness/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are required for this feature. Rust changes must pass `cargo test`
and `cargo clippy --all-targets -- -D warnings`.

**Organization**: Tasks are grouped by user story to enable independent implementation
and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g. `US1`, `US2`, `US3`)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Establish verification module entry points, fixtures, and spec-owned docs layout

- [X] T001 Create verification fixture directories and README seeds in `tests/fixtures/verification/README.md`, `tests/fixtures/verification/scenarios/.gitkeep`, `tests/fixtures/verification/generated_candidates/.gitkeep`, and `tests/fixtures/verification/artifacts/.gitkeep`
- [X] T002 Create verification module skeletons in `src/cli/verification.rs`, `src/core/verification_model.rs`, `src/core/verification_eval.rs`, `src/core/verification_generate.rs`, `src/io/libvirt.rs`, `src/io/guest.rs`, and `src/io/verification_artifacts.rs`
- [X] T003 Add optional verification guidance to `.specify/templates/spec-template.md` without making it the sole required generation input
- [X] T004 Wire verification modules into `src/cli/mod.rs`, `src/core/mod.rs`, `src/io/mod.rs`, `src/lib.rs`, and `src/bin/core-ops-verify.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the shared scenario model, boundary interfaces, and result contracts that all stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T005 Define shared verification domain types and enums in `src/core/types.rs` and `src/core/verification_model.rs`
- [X] T006 [P] Implement layered scenario schema parsing and validation helpers for intent, environment profiles, and harness-policy overrides in `src/core/verification_model.rs` and `tests/unit/test_verification_model.rs`
- [X] T007 [P] Implement pure run classification and assertion result evaluation helpers in `src/core/verification_eval.rs` and `tests/unit/test_verification_results.rs`
- [X] T008 [P] Implement libvirt and guest boundary traits plus command wrappers in `src/io/libvirt.rs`, `src/io/guest.rs`, and `src/core/boundaries.rs`
- [X] T009 Implement artifact bundle manifest and retention helpers in `src/io/verification_artifacts.rs` and `src/core/verification_model.rs`
- [X] T010 [P] Add layered scenario and run-result contract fixtures with environment profiles, omitted defaults, and semantic actions in `tests/fixtures/verification/scenarios/minimal-accepted.yaml`, `tests/fixtures/verification/scenarios/minimal-candidate.yaml`, and `tests/fixtures/verification/artifacts/run-result-passed.json`
- [X] T011 Add foundational contract coverage for layered scenario parsing, profile/default inheritance, semantic actions, and run-result serialization in `tests/integration/test_verification_contracts.rs`
- [X] T045 Evaluate reuse, extension, or replacement of `justfile`, `infra/ignition`, and `docs/development.md` CoreOS host-preparation workflow; record the chosen provisioning path and any required migration steps in `specs/008-e2e-verification-harness/plan.md` and `docs/development.md`
- [X] T046 [P] Define repository-evolution fixtures, environment-profile references, and revision-sequence domain types in `src/core/verification_model.rs`, `src/core/types.rs`, and `specs/008-e2e-verification-harness/data-model.md`
- [X] T047 [P] Add foundational contract fixtures for repository-evolution scenarios, environment/policy profiles, semantic actions, and public command-surface assertions in `tests/fixtures/verification/scenarios/`, `specs/008-e2e-verification-harness/contracts/scenario-schema.md`, and `specs/008-e2e-verification-harness/contracts/run-result-schema.md`
- [X] T053 [P] Define named environment-profile and harness-policy-profile contracts plus inheritance rules in `specs/008-e2e-verification-harness/data-model.md`, `specs/008-e2e-verification-harness/contracts/scenario-schema.md`, and `src/core/verification_model.rs`

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Run Disposable End-to-End Verification (Priority: P1) 🎯 MVP

**Goal**: Execute a declarative single-VM scenario against a disposable guest, classify the result deterministically, and retain diagnostic artifacts

**Independent Test**: Run an accepted scenario fixture through the dedicated verification tool entrypoint and confirm deterministic pass/fail classification, required artifacts, and default teardown versus debug retention behavior

### Tests for User Story 1 (REQUIRED) ⚠️

- [X] T012 [P] [US1] Add unit tests for semantic step sequencing, timeout enforcement, profile/default inheritance, and teardown/debug retention rules in `tests/unit/test_verification_execution.rs`
- [X] T013 [P] [US1] Add integration tests for accepted scenario execution with profile-based defaults, failed assertion classification, and artifact retention in `tests/integration/test_verification_execution.rs`
- [X] T014 [P] [US1] Add integration coverage for partial artifact collection failure preserving the primary scenario outcome in `tests/integration/test_verification_execution.rs`
- [X] T015 [P] [US1] Add entrypoint integration tests for local and debug verification runs in `tests/integration/test_verification_cli.rs`

### Implementation for User Story 1

- [X] T016 [P] [US1] Implement semantic scenario execution planning, ordered step orchestration, and default/profile expansion in `src/core/verification_eval.rs`
- [X] T017 [P] [US1] Implement libvirt workspace lifecycle and single-VM provisioning boundary in `src/io/libvirt.rs`
- [X] T018 [P] [US1] Implement guest readiness and command execution boundary in `src/io/guest.rs`
- [X] T019 [US1] Implement artifact collection, partial artifact failure handling, and default teardown/debug retention flow in `src/io/verification_artifacts.rs` and `src/cli/report.rs`
- [X] T020 [US1] Implement verification tool arguments and local/debug command surface for layered scenarios with profile/default expansion in `src/bin/core-ops-verify.rs` and `src/cli/verification.rs`
- [X] T021 [US1] Wire verification execution into the dedicated verification entrypoint in `src/bin/core-ops-verify.rs` and shared helpers in `src/cli/mod.rs`
- [X] T022 [US1] Emit deterministic human-readable verification summaries and failure diagnostics for the dedicated verification tool in `src/cli/report.rs` and `src/cli/verification.rs`

**Checkpoint**: User Story 1 should now be fully functional and independently testable

---

## Phase 4: User Story 2 - Derive Candidate Scenarios from Feature Specifications (Priority: P2)

**Goal**: Generate advisory candidate scenarios from feature specs and system semantics, then validate and filter them before they can enter review

**Independent Test**: Feed a feature specification and existing accepted corpus into the generation flow and confirm it emits schema-compliant candidates with taxonomy, rationale, and behavioral claims derived from the spec itself while rejecting malformed or redundant proposals

### Tests for User Story 2 (REQUIRED) ⚠️

- [X] T023 [P] [US2] Add unit tests for candidate normalization, duplicate coverage detection, and rejection criteria in `tests/unit/test_verification_generation.rs`
- [X] T024 [P] [US2] Add integration tests for advisory candidate generation from the feature spec itself, optional verification guidance handling, and deterministic validation flow in `tests/integration/test_verification_generation.rs`
- [X] T025 [P] [US2] Add entrypoint integration tests for candidate generation commands and review-ready output in `tests/integration/test_verification_cli.rs`
- [X] T051 [P] [US2] Add integration tests for feature-level and corpus-level missing scenario-class coverage reporting in `tests/integration/test_verification_generation.rs`

### Implementation for User Story 2

- [X] T026 [P] [US2] Implement candidate scenario data structures and review status handling in `src/core/verification_model.rs`
- [X] T027 [P] [US2] Implement spec-driven behavior extraction, optional verification-guidance handling, taxonomy mapping, and normalization pipeline in `src/core/verification_generate.rs`
- [X] T028 [US2] Implement candidate validation and rejection reasons against accepted corpus fixtures in `src/core/verification_generate.rs`
- [X] T029 [US2] Add candidate generation entrypoint flows and advisory output rendering for spec-driven generation in `src/bin/core-ops-verify.rs`, `src/cli/verification.rs`, and `src/cli/report.rs`
- [X] T030 [US2] Add candidate scenario fixtures, bug-reproduction promotion examples, and review examples in `tests/fixtures/verification/generated_candidates/feature-008-candidate.yaml`, `tests/fixtures/verification/generated_candidates/rejected-duplicate.yaml`, and `tests/fixtures/verification/README.md`
- [X] T052 [US2] Implement scenario-class coverage-gap analysis for feature specifications and accepted corpora in `src/core/verification_generate.rs`, `src/cli/verification.rs`, and `src/cli/report.rs`
- [X] T054 [US2] Implement candidate normalization onto layered scenarios with environment/policy profiles and semantic actions in `src/core/verification_generate.rs`, `tests/integration/test_verification_generation.rs`, and `tests/fixtures/verification/generated_candidates/`

**Checkpoint**: User Stories 1 and 2 should both work independently

---

## Phase 5: User Story 3 - Gate Revisions and Diagnose Regressions (Priority: P3)

**Goal**: Support deterministic CI gating and offline regression diagnosis with machine-readable run results tied to revision provenance

**Independent Test**: Run accepted scenarios in CI mode against a pinned revision and verify deterministic exit codes, machine-readable JSON output, retained artifact bundles, and explicit distinction among assertion, infrastructure, timeout, and harness failures

### Tests for User Story 3 (REQUIRED) ⚠️

- [X] T031 [P] [US3] Add contract tests for machine-readable run-result output in `tests/integration/test_verification_contracts.rs`
- [X] T032 [P] [US3] Add integration tests for VM-backed CI gating exit codes, repository-evolution regression reporting, and bug-reproduction reruns in `tests/integration/test_verification_execution.rs`, confirming accepted-corpus CI gating uses the authoritative disposable-VM execution path rather than synthetic-only helpers
- [X] T033 [P] [US3] Add entrypoint integration tests for non-interactive JSON output, accepted-corpus-only gating, selective scenario execution, and explicit CI-mode selection of the authoritative VM-backed execution path in `tests/integration/test_verification_cli.rs`
- [X] T048 [P] [US3] Add integration tests for public command-surface verification across supported human-readable, machine-readable, interactive, agent, and non-interactive interfaces in `tests/integration/test_verification_cli.rs` and `tests/integration/test_verification_execution.rs`

### Implementation for User Story 3

- [X] T034 [P] [US3] Implement run provenance fields, repository-evolution associations, and revision-under-test tracking in `src/core/types.rs` and `src/core/verification_model.rs`
- [X] T035 [P] [US3] Implement machine-readable run-result serialization and schema-aligned output builders in `src/bin/core-ops-verify.rs`, `src/cli/verification.rs`, and `src/cli/report.rs`
- [X] T036 [US3] Implement CI-mode execution flow, deterministic exit semantics, accepted-corpus-only gating, and selective scenario execution in `src/bin/core-ops-verify.rs` and `src/cli/verification.rs`
- [X] T037 [US3] Implement failure-specific artifact enrichment, regression comparison summaries, and bug-reproduction promotion support in `src/io/verification_artifacts.rs` and `src/cli/report.rs`, including the data and report surfaces needed to confirm a promoted bug reproduction is retained as an accepted permanent regression scenario
- [X] T049 [US3] Implement public command-surface contract verification and coarse timing-guardrail assertions for guest readiness, scenario timeout enforcement, and supported public interfaces in `src/core/verification_eval.rs`, `src/cli/verification.rs`, and `src/cli/report.rs`
- [X] T038 [US3] Add quickstart-backed verification examples for local, debug, and CI flows in `tests/integration/test_quickstart_verification.rs`, demonstrating VM-backed execution as the normative path and treating synthetic helpers as internal-only validation support

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final documentation, validation gates, and cross-story hardening

- [X] T039 [P] Add opt-in libvirt-backed smoke test coverage in `tests/integration/test_verification_smoke.rs`
- [X] T040 [P] Document dedicated verification tool flows and scenario authoring guidance in `docs/development.md` and `specs/008-e2e-verification-harness/quickstart.md`
- [X] T041 [P] Add release-version-policy review notes for new scenario and run-result contracts plus the dedicated verification-tool entrypoint in `specs/008-e2e-verification-harness/plan.md` and `specs/008-e2e-verification-harness/contracts/run-result-schema.md`
- [X] T042 [P] Add additional repeatability and determinism coverage in `tests/integration/test_verification_execution.rs` and `tests/unit/test_verification_results.rs`
- [X] T050 [P] Document repository-evolution authoring, bug-reproduction promotion, selective developer workflows, and the acceptance workflow for turning a reproduced bug into a permanent accepted regression scenario in `docs/development.md`, `specs/008-e2e-verification-harness/quickstart.md`, and `tests/fixtures/verification/README.md`
- [X] T055 [P] Document authorable minimal-scenario conventions, profile usage, omitted defaults, and semantic step actions in `docs/development.md`, `specs/008-e2e-verification-harness/quickstart.md`, and `tests/fixtures/verification/README.md`
- [X] T043 Run `cargo test` and capture the result in the implementation record for `specs/008-e2e-verification-harness/tasks.md`
- [X] T044 Run `cargo clippy --all-targets -- -D warnings` and capture the result in the implementation record for `specs/008-e2e-verification-harness/tasks.md`

### Implementation Record

- 2026-04-03: `cargo test verification_execution -- --nocapture` passed after completing the env-backed disposable-VM execution path and real guest diagnostics.
- 2026-04-03: `cargo clippy --all-targets -- -D warnings` passed after the same env-backed runtime and artifact changes.
- 2026-04-03: `cargo test verification -- --nocapture` passed after adding machine-readable `verification_run` JSON output and contract coverage.
- 2026-04-03: `cargo clippy --all-targets -- -D warnings` passed after the same machine-readable output changes.
- 2026-04-05: `cargo test verification -- --nocapture` passed after adding CI-mode accepted-corpus execution, `--scenario-id` filtering, and deterministic run exit semantics.
- 2026-04-05: `cargo clippy --all-targets -- -D warnings` passed after the same CI/corpus execution changes.
- 2026-04-06: `cargo test verification -- --nocapture` passed after adding batch provenance fields, per-scenario revision tracking, and suite bundle revision indexing for US3.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same US3 provenance changes.
- 2026-04-06: `cargo test verification -- --nocapture` passed after adding quickstart-backed verification documentation tests for local, debug, and CI flows.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same quickstart-backed verification test additions.
- 2026-04-06: `cargo test verification -- --nocapture` passed after adding repeatability and determinism coverage for repeated scenario execution and run-outcome precedence.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same repeatability and determinism coverage additions.
- 2026-04-06: `cargo test verification -- --nocapture` passed after confirming CI gating exit-code, repository-evolution provenance, and focused regression-rerun integration coverage for US3.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same US3 CI/regression integration coverage confirmation.
- 2026-04-06: `cargo test verification -- --nocapture` passed after adding public command-surface assertions, coarse timing guardrails, and scenario-timeout enforcement coverage for US3.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same public command-surface and timing-guardrail implementation.
- 2026-04-06: `cargo test verification -- --nocapture` passed after adding failure-summary, regression-summary, and promotion-status artifact/report enrichment for failing accepted regression scenarios.
- 2026-04-06: `cargo clippy --all-targets -- -D warnings` passed after the same regression artifact/report enrichment changes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1: Setup**: No dependencies - can start immediately
- **Phase 2: Foundational**: Depends on Phase 1 completion - blocks all user stories
  - Foundational provisioning-path decision `T045` must complete before story implementation treats provisioning as settled
  - Foundational repository-evolution and contract tasks `T046` and `T047` must complete before story implementation treats revision-history coverage as settled
- **Phase 3: User Story 1**: Depends on Phase 2 completion
- **Phase 4: User Story 2**: Depends on Phase 2 completion and reuses accepted scenario parsing from US1 foundations
- **Phase 5: User Story 3**: Depends on Phase 2 completion and reuses execution outputs from US1
- **Phase 6: Polish**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational - MVP, no dependency on other stories
- **US2 (P2)**: Can start after Foundational - independent from US1 execution flow except for shared scenario contracts
- **US3 (P3)**: Can start after Foundational - depends on shared execution/result infrastructure and benefits from US1 completion for end-to-end coverage

### Within Each User Story

- Tests MUST be written and fail before implementation
- Model and pure logic tasks precede orchestration and CLI tasks
- Boundary adapters precede command wiring
- Story-specific reporting follows core behavior implementation

---

## Parallel Opportunities

- **Setup**: `T001` and `T002` can proceed in parallel once the plan is accepted
- **Foundational**: `T006`, `T007`, `T008`, `T010`, `T046`, `T047`, and `T053` are parallelizable after `T005`; `T045` should run early in Phase 2 because it can affect provisioning direction
- **US1**: `T012`, `T013`, `T014`, and `T015` can be authored together once layered scenario contracts are stable; `T017` and `T018` can proceed in parallel after the foundational boundary types exist
- **US2**: `T023`, `T024`, `T025`, and `T051` can proceed together; `T026` and `T027` can run in parallel, followed by `T054`
- **US3**: `T031`, `T032`, `T033`, and `T048` can proceed together; `T034` and `T035` can run in parallel
- **Polish**: `T039`, `T040`, `T041`, `T042`, and `T050` can proceed in parallel before final validation gates

---

## Parallel Example: User Story 1

```bash
# Write the failing US1 tests together:
Task: "Add unit tests for step sequencing, timeout enforcement, and teardown/debug retention rules in tests/unit/test_verification_execution.rs"
Task: "Add integration tests for accepted scenario execution, failed assertion classification, and artifact retention in tests/integration/test_verification_execution.rs"
Task: "Add integration coverage for partial artifact collection failure preserving the primary scenario outcome in tests/integration/test_verification_execution.rs"
Task: "Add CLI integration tests for local and debug verification runs in tests/integration/test_verification_cli.rs"

# Build the boundary adapters together after tests exist:
Task: "Implement libvirt workspace lifecycle and single-VM provisioning boundary in src/io/libvirt.rs"
Task: "Implement guest readiness and command execution boundary in src/io/guest.rs"
```

## Parallel Example: User Story 2

```bash
# Create the failing US2 tests together:
Task: "Add unit tests for candidate normalization, duplicate coverage detection, and rejection criteria in tests/unit/test_verification_generation.rs"
Task: "Add integration tests for advisory candidate generation from the feature spec itself, optional verification guidance handling, validation flow, and scenario-class coverage reporting in tests/integration/test_verification_generation.rs"

# Build generation logic in parallel:
Task: "Implement candidate scenario data structures and review status handling in src/core/verification_model.rs"
Task: "Implement spec-driven behavior extraction, taxonomy mapping, optional verification guidance handling, normalization, and scenario-class coverage analysis in src/core/verification_generate.rs"
Task: "Normalize generated candidates onto layered scenarios with environment/policy profiles and semantic actions in src/core/verification_generate.rs"
```

## Parallel Example: User Story 3

```bash
# Add the CI-facing tests together:
Task: "Add contract tests for machine-readable run-result output in tests/integration/test_verification_contracts.rs"
Task: "Add integration tests for CI gating exit codes and revision-associated regression reporting in tests/integration/test_verification_execution.rs"
Task: "Add CLI integration tests for non-interactive JSON output and accepted-corpus-only gating in tests/integration/test_verification_cli.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. Validate scenario execution, artifact retention, and debug teardown behavior
5. Stop for review before adding generation or CI gating work

### Incremental Delivery

1. Foundation first: schema, result model, boundary traits, and artifact manifest
2. Add US1 to prove executable runtime verification on disposable guests
3. Add US2 to derive advisory candidate scenarios from specifications
4. Add US3 to make the harness useful as a CI gate and regression-diagnosis surface
5. Add opt-in smoke coverage, docs, and mandatory validation gates

### Suggested MVP Scope

- **MVP**: Phase 1 + Phase 2 + Phase 3 (User Story 1)
- This delivers real disposable verification, deterministic pass/fail
  classification, required artifacts, and debug retention without waiting for
  LLM-assisted generation or CI gating

---

## Notes

- All tasks follow the required checklist format with IDs, optional `[P]`, story labels where required, and exact file paths
- Tests are included for every user story because the spec and constitution require explicit behavioral validation
- User stories remain independently testable even though they share foundational scenario contracts
