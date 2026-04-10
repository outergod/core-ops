# Tasks: SemVer and Changelog Governance

**Input**: Design documents from `/specs/011-semver-changelog-governance/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Tests are REQUIRED for this feature. Rust changes must include
`cargo test` and `cargo clippy --all-targets -- -D warnings`.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this belongs to (e.g. `[US1]`, `[US2]`, `[US3]`)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the governance artifact locations and baseline repository scaffolding used by later stories.

- [X] T001 Create release fragment directory scaffolding and document the `changes/<change-id>.md` naming convention in changes/README.md
- [X] T002 [P] Add fixture placeholders for release-governance contract tests in tests/fixtures/release_governance/
- [X] T003 [P] Add feature documentation references for release governance in docs/development.md

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core release-governance primitives required by all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 Implement release fragment and governance evaluation data types in src/core/release_governance.rs
- [X] T005 [P] Implement release fragment loading and repository file scanning helpers in src/io/release_governance.rs
- [X] T006 [P] Implement SemVer parsing and comparison helpers for Cargo.toml version deltas in src/core/release_governance.rs
- [X] T007 Implement governance report rendering and machine-readable output helpers in src/cli/report.rs
- [X] T008 Add the `core-ops-release` helper binary entrypoint in src/bin/core-ops-release.rs and wire it to shared governance evaluation code
- [X] T009 Add baseline unit coverage for fragment parsing, exemption rules, and bump comparison in tests/unit/test_release_governance.rs

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Reject Incomplete Release Metadata (Priority: P1) 🎯 MVP

**Goal**: Releasable pull requests fail when required version, release-fragment, or generated changelog artifacts are missing.

**Independent Test**: Run the governance validation against releasable and exempt fixture change sets and verify missing metadata fails with explicit reasons while exempt-only no-fragment changes pass.

### Tests for User Story 1 ⚠️

- [X] T010 [P] [US1] Add integration tests for missing version, missing fragment, mixed releasable/exempt deltas, and exempt-only no-fragment pass behavior in tests/integration/test_release_governance_validation.rs
- [X] T011 [P] [US1] Add workflow contract tests for the stable PR governance job name, pull-request trigger, and required-check suitability in tests/integration/test_release_governance_workflow.rs

### Implementation for User Story 1

- [X] T012 [US1] Implement release classification policy evaluation for exempt, releasable, and mixed change sets in src/core/release_governance.rs
- [X] T013 [US1] Implement governance failure reporting for missing artifacts and mixed exempt/releasable deltas in src/core/release_governance.rs
- [X] T014 [US1] Implement helper-binary command execution for governance validation in src/bin/core-ops-release.rs
- [X] T015 [US1] Add repository test fixtures for releasable, exempt, and mixed change sets in tests/fixtures/release_governance/
- [X] T016 [US1] Add CI workflow step or dedicated workflow contract updates for the required governance check in .github/workflows/ci.yml

**Checkpoint**: User Story 1 should reject incomplete release metadata and allow exempt-only changes independently

---

## Phase 4: User Story 2 - Enforce Correct SemVer Intent (Priority: P2)

**Goal**: The governance check computes the required bump and rejects mismatched release intent.

**Independent Test**: Validate patch, minor, major, highest-bump, and accepted-verification-corpus fixture change sets and confirm the check reports the effective required bump deterministically.

### Tests for User Story 2 ⚠️

- [X] T017 [P] [US2] Add integration tests for patch, minor, major, highest-bump mismatches, and accepted verification corpus patch-floor cases in tests/integration/test_release_governance_semver.rs
- [X] T018 [P] [US2] Add unit tests for SemVer decision rules and highest-bump precedence in tests/unit/test_release_governance.rs

### Implementation for User Story 2

- [X] T019 [US2] Implement SemVer decision-table evaluation for patch, minor, and major triggers in src/core/release_governance.rs
- [X] T020 [US2] Implement highest-bump aggregation across multiple releasable deltas in src/core/release_governance.rs
- [X] T021 [US2] Implement mismatch diagnostics that report declared vs required bump in src/core/release_governance.rs
- [X] T022 [US2] Add fixture change sets covering additive, breaking, mixed-impact, and accepted verification corpus changes in tests/fixtures/release_governance/

**Checkpoint**: User Stories 1 and 2 should both work independently, with deterministic SemVer enforcement

---

## Phase 5: User Story 3 - Preserve Fast Paths for Exempt Changes (Priority: P3)

**Goal**: Exempt-only pull requests pass without metadata, while metadata-only PRs require explicit `release_preparation: true` intent.

**Independent Test**: Validate docs-only and formatting-only change sets with no fragment, version, or changelog metadata, plus metadata-only and release-preparation fixture change sets, and confirm each follows the specified policy.

### Tests for User Story 3 ⚠️

- [X] T023 [P] [US3] Add integration tests for exempt-only no-fragment passes and metadata-only release-preparation behavior in tests/integration/test_release_governance_exemptions.rs
- [X] T024 [P] [US3] Add unit tests for `release_preparation` fragment field handling in tests/unit/test_release_governance.rs

### Implementation for User Story 3

- [X] T025 [US3] Implement explicit exemption-rule evaluation for docs, comments, formatting, and context-dependent workflow changes in src/core/release_governance.rs
- [X] T026 [US3] Implement metadata-only release-preparation handling keyed by `release_preparation: true` in src/core/release_governance.rs
- [X] T027 [US3] Add fixture change sets for exempt-only no-fragment and release-preparation scenarios in tests/fixtures/release_governance/
- [X] T028 [US3] Document contributor guidance for `changes/<change-id>.md` fragments and release-preparation changes in AGENTS.md and docs/development.md and validate it in tests/integration/test_release_governance_docs.rs

**Checkpoint**: All user stories should now be independently functional

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finalize changelog generation, docs, and validation paths across the full feature, including the same-change-set release artifacts required for the implementing pull request.

- [X] T029 [P] Implement changelog generation from approved `changes/<change-id>.md` release fragments in src/bin/core-ops-release.rs
- [X] T030 Update CHANGELOG.md generation guidance and release-history expectations in README.md and docs/development.md
- [X] T031 [P] Add integration tests covering generated changelog alignment with approved fragments in tests/integration/test_release_governance_changelog.rs
- [X] T032 [P] Run `cargo test` and capture any required remediation in specs/011-semver-changelog-governance/quickstart.md
- [X] T033 [P] Run `cargo clippy --all-targets -- -D warnings` and capture any required remediation in specs/011-semver-changelog-governance/quickstart.md
- [X] T034 Verify reviewer-facing governance output explains classification, required bump, missing artifacts, and applied rules across release-governance CLI flows in tests/integration/test_release_governance_validation.rs
- [X] T035 Evaluate and record the highest required SemVer bump (`patch`, `minor`, or `major`) for the feature itself in changes/011-semver-changelog-governance.md
- [X] T036 Update the machine-checkable release fragment for this feature in changes/011-semver-changelog-governance.md
- [X] T037 Update `Cargo.toml` to the required release version for this feature in Cargo.toml as part of the same implementing change set
- [X] T038 Regenerate and commit `CHANGELOG.md` from approved fragments in the same change set as the fragment and version updates in CHANGELOG.md
- [X] T039 Run quickstart validation using specs/011-semver-changelog-governance/quickstart.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can proceed in priority order or in parallel once foundation is complete
- **Polish (Phase 6)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Starts after Foundational - no dependency on later stories
- **User Story 2 (P2)**: Starts after Foundational - depends logically on governance command/reporting infrastructure from US1
- **User Story 3 (P3)**: Starts after Foundational - depends logically on classification primitives from US1

### Within Each User Story

- Tests MUST be written and fail before implementation
- Core policy/data changes before CLI/report integration
- Fixture updates before final validation
- Story complete before moving to the next priority if working sequentially

### Parallel Opportunities

- T002 and T003 can run in parallel
- T005 and T006 can run in parallel after T004
- Story-level test tasks marked [P] can run in parallel
- Fixture authoring tasks for distinct stories can run in parallel
- Final validation tasks T032 and T033 can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch US1 tests together:
Task: "Add integration tests for missing version, missing fragment, and exempt-only PR behavior in tests/integration/test_release_governance_validation.rs"
Task: "Add workflow contract tests for the required PR governance check in tests/integration/test_release_governance_workflow.rs"

# Launch foundational helpers together once core types exist:
Task: "Implement release fragment loading and repository file scanning helpers in src/io/release_governance.rs"
Task: "Implement SemVer parsing and comparison helpers for Cargo.toml version deltas in src/core/release_governance.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Confirm missing metadata is rejected and exempt-only changes pass
5. Demo the governance command and CI contract

### Incremental Delivery

1. Complete Setup + Foundational → governance infrastructure ready
2. Add User Story 1 → validate metadata completeness enforcement
3. Add User Story 2 → validate SemVer mismatch and highest-bump enforcement
4. Add User Story 3 → validate exemption and release-preparation behavior
5. Finish changelog generation and cross-cutting validation

### Parallel Team Strategy

With multiple developers:

1. Complete Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1
   - Developer B: User Story 2
   - Developer C: User Story 3
3. Integrate in Phase 6 for changelog generation, docs, and final validation

---

## Notes

- [P] tasks = different files, no dependencies
- [US#] labels map tasks to user stories for traceability
- Each user story is independently completable and testable
- Tests are mandatory for this feature
- Include release-intent artifact, `Cargo.toml`, and generated `CHANGELOG.md` updates in the same pull request whenever work is releasable
- Verify tests fail before implementing
- Avoid vague tasks and cross-story dependencies that break independence
