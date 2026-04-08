# Tasks: Distribution Readiness

**Input**: Design documents from `/specs/010-distribution-readiness/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are required for this feature. Rust-facing changes must pass
`cargo test` and `cargo clippy --all-targets -- -D warnings`.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the public-distribution scaffolding and shared fixtures the rest of the feature will build on.

- [X] T001 Create repository-root public document scaffolds in `README.md`, `CHANGELOG.md`, `LICENSE`, and `CODE_OF_CONDUCT.md`
- [X] T002 Create distribution fixture scaffolds in `tests/fixtures/distribution/README.md`, `tests/fixtures/distribution/entrypoint-snapshot.md`, and `tests/fixtures/distribution/release-gate-environment.json`
- [X] T003 [P] Create workflow scaffolds in `.github/workflows/ci.yml`, `.github/workflows/e2e-gate.yml`, and `.github/workflows/release-binary.yml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared release identity, credibility, and validation infrastructure that blocks all user stories.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 Define shared distribution-readiness models and fixtures in `src/build_info.rs`, `src/core/verification_model.rs`, and `tests/fixtures/distribution/release-gate-environment.json`
- [X] T005 [P] Add shared unit coverage for release identity, credibility, and verification-environment identity in `tests/unit/test_verification_model.rs`
- [X] T006 [P] Add shared integration coverage for public version/report parity in `tests/integration/test_status_contract.rs`
- [X] T007 Implement shared humane and machine-readable release reporting in `src/cli/report.rs` and `src/io/verification_artifacts.rs`
- [X] T008 Implement shared release identity visibility helpers in `src/build_info.rs`, `src/cli/common.rs`, and `src/cli/status.rs`
- [X] T009 Create release metadata and credibility-surface source contracts in `tests/fixtures/distribution/release-metadata.json` and `src/build_info.rs`
- [X] T010 Document release-version-policy expectations for this feature in `docs/development.md`

**Checkpoint**: Foundation ready; user story work can now begin.

---

## Phase 3: User Story 1 - Understand And Trust The Project (Priority: P1) 🎯 MVP

**Goal**: Give a competent outsider a clear, trustworthy entrypoint that explains fit, limits, licensing, conduct, and support boundaries without maintainer help.

**Independent Test**: Review the public entrypoint and confirm it includes framing, goals, non-goals, support boundary, AI authorship disclosure, trust story, stable credibility surface, license, code of conduct, and logo placeholder in consistently discoverable locations.

### Tests for User Story 1 (REQUIRED) ⚠️

- [X] T011 [P] [US1] Add entrypoint contract coverage in `tests/integration/test_distribution_entrypoint.rs`
- [X] T012 [P] [US1] Add public-document discoverability coverage in `tests/integration/test_distribution_materials.rs`
- [X] T013 [P] [US1] Add credibility-surface consistency coverage in `tests/integration/test_distribution_credibility.rs`

### Implementation for User Story 1

- [X] T014 [US1] Write the public project entrypoint in `README.md` with framing, goals, non-goals, support boundary, unsupported container execution, and trust story
- [X] T015 [P] [US1] Add the AGPLv3+ license text in `LICENSE` and align package metadata in `Cargo.toml`
- [X] T016 [P] [US1] Add the public community behavior document in `CODE_OF_CONDUCT.md`
- [X] T017 [P] [US1] Add the project logo placeholder asset in `docs/logo-placeholder.svg`
- [X] T018 [US1] Add the stable credibility surface, AI authorship disclosure, and entrypoint links to `README.md`
- [X] T019 [US1] Record the supported/unsupported system matrix and public-support rationale in `README.md` and `docs/development.md`

**Checkpoint**: User Story 1 should now be independently understandable and reviewable by an outside evaluator.

---

## Phase 4: User Story 2 - Install And Verify A Release Candidate (Priority: P2)

**Goal**: Provide a binary-only release path with a cold-start install flow, a reproducible operator verification flow, and a documented authoritative verification environment.

**Independent Test**: Follow the binary install path on a fresh Fedora CoreOS environment, run the documented first command and smoke test, perform the minimal operator verification flow, and confirm release-gate outputs identify the same release and verification environment.

### Tests for User Story 2 (REQUIRED) ⚠️

- [X] T020 [P] [US2] Add release-gate decision and environment-identity coverage in `tests/integration/test_distribution_release_gate.rs`
- [X] T021 [P] [US2] Add binary install-and-verify flow coverage for supported architectures and canonical service/timer integration in `tests/integration/test_distribution_installation.rs`
- [X] T022 [P] [US2] Add version/provenance visibility coverage in `tests/integration/test_distribution_version_visibility.rs`
- [X] T023 [P] [US2] Add binary release license and architecture propagation coverage in `tests/integration/test_distribution_release_artifacts.rs`
- [X] T024 [P] [US2] Add CLI license visibility coverage in `tests/integration/test_distribution_cli_license.rs`

### Implementation for User Story 2

- [X] T025 [US2] Implement binary version and source/spec identity stamping in `src/build_info.rs`, `src/main.rs`, and `src/bin/core-ops-verify.rs`
- [X] T026 [US2] Expose release identity through operator-facing surfaces in `src/cli/status.rs`, `src/cli/explain.rs`, `src/cli/report.rs`, and `src/cli/args.rs`
- [X] T027 [US2] Extend verification artifacts with release-gate and authoritative-environment identity in `src/cli/verification.rs` and `src/io/verification_artifacts.rs`
- [X] T028 [US2] Implement the public CI and protected authoritative E2E gate workflows in `.github/workflows/ci.yml` and `.github/workflows/e2e-gate.yml`
- [X] T029 [P] [US2] Implement multi-architecture binary publication workflow and release metadata generation in `.github/workflows/release-binary.yml`
- [X] T030 [US2] Add AGPLv3+ license references to binary release outputs, release metadata, and a discoverable CLI surface in `.github/workflows/release-binary.yml`, `README.md`, and `src/cli/args.rs`
- [X] T031 [US2] Document the supported `x86_64` and `aarch64` binary acquisition, installation, first command, smoke-test flow, and canonical `core-ops.service`/`core-ops.timer` path in `README.md`
- [X] T032 [US2] Document the minimal operator-facing verification flow and cold-start expectations in `README.md` and `specs/010-distribution-readiness/quickstart.md`
- [X] T033 [US2] Document the authoritative verification environment and runner-drift contract in `docs/development.md` and `README.md`

**Checkpoint**: User Story 2 should now support independent install, first-run verification, and release-gate attribution without relying on maintainer-only infrastructure.

---

## Phase 5: User Story 3 - Diagnose Failures And Track Changes Safely (Priority: P3)

**Goal**: Make failures understandable to outsiders and keep release history and public materials auditable over time.

**Independent Test**: Review a failed release-gate or operator-facing failure path, confirm the error is actionable and versioned, and verify that the changelog and release materials explain externally relevant changes.

### Tests for User Story 3 (REQUIRED) ⚠️

- [X] T034 [P] [US3] Add failure-ergonomics and report-output coverage in `tests/integration/test_distribution_failures.rs`
- [X] T035 [P] [US3] Add changelog and public-material consistency coverage in `tests/integration/test_distribution_history.rs`

### Implementation for User Story 3

- [X] T036 [US3] Backfill and structure the release history in `CHANGELOG.md` using Keep a Changelog format
- [X] T037 [US3] Improve actionable, versioned diagnostics in `src/cli/diagnostics.rs` and `src/cli/report.rs`
- [X] T038 [US3] Expose release identity and troubleshooting guidance in failure-oriented surfaces in `src/cli/verification.rs` and `src/cli/common.rs`
- [X] T039 [US3] Document operator audit, recovery, and failure-handling guidance in `README.md` and `docs/development.md`
- [X] T040 [US3] Add release-gate failure summaries and drift-reporting guidance to `.github/workflows/ci.yml`, `.github/workflows/e2e-gate.yml`, and `docs/development.md`

**Checkpoint**: All user stories should now be independently functional and externally reviewable.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final consistency, validation, and release-readiness checks that span multiple user stories.

- [X] T041 [P] Refresh the quickstart walkthrough and expected results in `specs/010-distribution-readiness/quickstart.md`
- [X] T042 [P] Align the public distribution proposal and development guidance in `docs/distribution-readiness-proposal.md` and `docs/development.md`
- [X] T043 [P] Add or update quickstart validation coverage in `tests/integration/test_quickstart_verification.rs`
- [X] T044 Run `cargo test` and record any required follow-up under a Validation Follow-Up subsection in `docs/development.md`
- [X] T045 Run `cargo clippy --all-targets -- -D warnings` and record any required follow-up under a Validation Follow-Up subsection in `docs/development.md`
- [X] T046 Verify release-version-policy impact and update `CHANGELOG.md` and `Cargo.toml` as needed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion; blocks all user stories
- **User Stories (Phases 3-5)**: Depend on Foundational completion
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Starts after Foundational; no dependency on other user stories
- **User Story 2 (P2)**: Starts after Foundational; may reuse US1 entrypoint material but remains independently testable
- **User Story 3 (P3)**: Starts after Foundational; builds on release/version surfaces introduced earlier but remains independently testable

### Within Each User Story

- Tests must be written and fail before implementation
- Public contracts and discoverability surfaces before related polish
- Shared reporting/version surfaces before workflow or documentation claims that rely on them
- Story-specific documentation updates before final cross-cutting validation

## Parallel Opportunities

- `T002` and `T003` can run in parallel after `T001`
- `T005` and `T006` can run in parallel after `T004`
- `T015`, `T016`, and `T017` can run in parallel after `T014`
- `T020`, `T021`, `T022`, `T023`, and `T024` can run in parallel at the start of US2
- `T028` and `T029` can run in parallel once shared release identity/reporting work is in place
- `T034` and `T035` can run in parallel at the start of US3
- `T041`, `T042`, and `T043` can run in parallel during polish

## Parallel Example: User Story 1

```bash
# Launch the User Story 1 tests together:
Task: "Add entrypoint contract coverage in tests/integration/test_distribution_entrypoint.rs"
Task: "Add public-document discoverability coverage in tests/integration/test_distribution_materials.rs"
Task: "Add credibility-surface consistency coverage in tests/integration/test_distribution_credibility.rs"

# Launch the independent public documents together:
Task: "Add the AGPLv3+ license text in LICENSE and align package metadata in Cargo.toml"
Task: "Add the public community behavior document in CODE_OF_CONDUCT.md"
Task: "Add the project logo placeholder asset in docs/logo-placeholder.svg"
```

## Parallel Example: User Story 2

```bash
# Launch the User Story 2 tests together:
Task: "Add release-gate decision and environment-identity coverage in tests/integration/test_distribution_release_gate.rs"
Task: "Add binary install-and-verify flow coverage in tests/integration/test_distribution_installation.rs"
Task: "Add version/provenance visibility coverage in tests/integration/test_distribution_version_visibility.rs"
Task: "Add binary release license propagation coverage in tests/integration/test_distribution_release_artifacts.rs"
Task: "Add CLI license visibility coverage in tests/integration/test_distribution_cli_license.rs"

# Launch workflow implementation in parallel once shared release reporting exists:
Task: "Implement the public CI and protected authoritative E2E gate workflows in .github/workflows/ci.yml and .github/workflows/e2e-gate.yml"
Task: "Implement binary publication workflow and release metadata generation in .github/workflows/release-binary.yml"
```

## Parallel Example: User Story 3

```bash
# Launch the User Story 3 tests together:
Task: "Add failure-ergonomics and report-output coverage in tests/integration/test_distribution_failures.rs"
Task: "Add changelog and public-material consistency coverage in tests/integration/test_distribution_history.rs"
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. Stop and validate the public entrypoint independently

### Incremental Delivery

1. Setup and foundational work establish shared release identity/reporting
2. Deliver User Story 1 to make the project understandable and trustworthy
3. Deliver User Story 2 to make the binary distribution and install/verify path real
4. Deliver User Story 3 to make failures and release history operationally credible
5. Finish with cross-cutting validation and release-policy checks

### Suggested MVP Scope

- Phase 1
- Phase 2
- Phase 3 only

## Notes

- All tasks follow the required checklist format.
- All user-story tasks include `[US#]` labels.
- Every task includes exact file paths.
- Tests are included for each user story because this feature changes public contracts and Rust-facing behavior.
