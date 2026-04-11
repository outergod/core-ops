# Tasks: Unify CI Validation And Release Publication

**Input**: Design documents from `specs/012-unify-ci-release/`  
**Prerequisites**: plan.md ✓, spec.md ✓, research.md ✓, data-model.md ✓, contracts/ci-workflow.md ✓

**Tests**: No TDD; no new Rust logic. Updated integration tests run under existing
`cargo test`. No clippy exception — standard gates apply to modified test files.

**Note**: No Phase 1 Setup or Phase 2 Foundational phases are required. The existing
`ci` job in `.github/workflows/ci.yml` is the foundation and requires no changes.
All work is modification of existing files or addition of new governance artifacts.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 3: User Story 1 — PR Validation With Build Artifacts (Priority: P1) 🎯 MVP

**Goal**: Every PR gets automated governance feedback AND downloadable workflow artifacts
for both x86_64 and aarch64 targets, visible in the Actions checks panel.

**Independent Test**: Open a PR with a compliant change fragment. After CI completes,
confirm `core-ops-binary-release-x86_64-unknown-linux-gnu` and
`core-ops-binary-release-aarch64-unknown-linux-gnu` appear as downloadable artifacts
under the `Build Release Binaries` job. See quickstart.md §PR validation.

### Implementation for User Story 1

- [ ] T001 [US1] Add `build` matrix job to `.github/workflows/ci.yml`:
  `needs: ci`, matrix `[x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]`,
  `fail-fast: false`, env `CORE_OPS_BUILD_SPEC_CONTEXT: "specs/010-distribution-readiness/spec.md"`,
  cross-toolchain install step (aarch64 only), `cargo build --release --locked --target "${{ matrix.target }}"`,
  full packaging into `dist/` (binary, systemd units, LICENSE, CHANGELOG.md, README.md,
  tarball, SHA256SUMS, release-metadata.json with version from `grep '^version' Cargo.toml`),
  `actions/upload-artifact@v4` with `name: core-ops-binary-release-${{ matrix.target }}` and `path: dist/`;
  set `release_identity` to `v${version}` where version is derived from `grep '^version' Cargo.toml`

**Checkpoint**: Push a draft PR. Confirm the `Build Release Binaries (x86_64-unknown-linux-gnu)`
and `Build Release Binaries (aarch64-unknown-linux-gnu)` jobs appear with downloadable artifacts.

---

## Phase 4: User Story 2 — Canonical Release On Trunk Push (Priority: P2)

**Goal**: A push to `master` with a new Cargo.toml version automatically publishes
a GitHub Release with the version tag, CHANGELOG-sourced release notes, and all
binary assets — no manual trigger.

**Independent Test**: Merge a compliant PR to master. Confirm a `v<version>` GitHub
Release appears with binary assets and CHANGELOG-sourced body. See quickstart.md §Master push.

### Implementation for User Story 2

- [ ] T002 [US2] Add `release` job to `.github/workflows/ci.yml`:
  `needs: build`, `if: github.ref == 'refs/heads/master' && github.event_name == 'push'`,
  `permissions: contents: write`, `fetch-depth: 0`.
  Steps: (1) derive `version` from `grep '^version' Cargo.toml`, derive `tag=v${version}`;
  (2) duplicate-tag check via `git ls-remote --tags origin "refs/tags/${tag}"` — explicit
  `exit 1` with actionable message if found; (3) extract release notes into temp file via
  `awk "/^## \[${version}\]/{found=1; next} found && /^## \[/{exit} found{print}" CHANGELOG.md`;
  (4) `actions/download-artifact@v4` with `merge-multiple: true` to `dist/`;
  (5) `gh release create "${tag}" --title "core-ops ${tag}" --notes-file <tmp>`
  with assets: `dist/core-ops-linux-amd64`, `dist/core-ops-linux-arm64`,
  `dist/core-ops-linux-amd64.tar.gz`, `dist/core-ops-linux-arm64.tar.gz`,
  `dist/SHA256SUMS-amd64`, `dist/SHA256SUMS-arm64`

**Checkpoint**: After merging a version-bumping PR, confirm the GitHub Release exists
with all six binary assets and the correct body extracted from `CHANGELOG.md`.

---

## Phase 5: User Story 3 — Retire Separate Release Workflow (Priority: P3)

**Goal**: `release-binary.yml` no longer exists. A contributor reading the workflow
directory sees only `ci.yml` and `e2e-gate.yml`.

**Independent Test**: Run `ls .github/workflows/` — confirm `release-binary.yml` absent.
Run `cargo test test_distribution_release` — all tests pass against the unified `ci.yml`.

### Implementation for User Story 3

- [ ] T003 [P] [US3] Delete `.github/workflows/release-binary.yml`

- [ ] T004 [P] [US3] Update `tests/integration/test_distribution_release_artifacts.rs`:
  Rename function `release_binary_workflow_includes_license_and_metadata_outputs` to
  `release_workflow_includes_license_and_metadata_outputs`;
  change `read_to_string(... "release-binary.yml")` to `read_to_string(... "ci.yml")`;
  replace snippet `"release_identity=\"${{ github.event.release.tag_name }}\""` with
  `"grep '^version' Cargo.toml"`.
  Add new test function `unified_release_job_is_gated_to_master_push` asserting `ci.yml` contains:
  `"refs/heads/master"`, `"git ls-remote --tags origin"`, `"gh release create"`, `"contents: write"`

**Checkpoint**: `cargo test test_distribution_release` passes with zero failures.
`ls .github/workflows/` shows only `ci.yml` and `e2e-gate.yml`.

---

## Phase 6: User Story 4 — Live Credibility Badges In README (Priority: P4)

**Goal**: The README Credibility section shows live badges for CI status, E2E gate
status, and latest published release version. The static `0.7.0-dev` string is gone.

**Independent Test**: View `README.md` on GitHub (master branch) after at least one
release has been published. Confirm three badges render with live data.
See quickstart.md §Live badges.

### Implementation for User Story 4

- [ ] T005 [US4] Update `README.md` Credibility section:
  Replace the static table with a badge row for CI status
  (`https://github.com/outergod/core-ops/actions/workflows/ci.yml/badge.svg?branch=master`),
  E2E gate status
  (`https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml/badge.svg`),
  and latest release version
  (`https://img.shields.io/github/v/release/outergod/core-ops`).
  Each badge must be a live-linked markdown image.
  Retain static rows for Published artifacts and Verification environment where no
  live-data equivalent exists. Update the explanatory paragraph to reflect that
  badges are now live.

**Checkpoint**: README renders on GitHub with three live badge images in the
Credibility section.

---

## Phase 7: Polish & Release Governance Artifacts

**Purpose**: Release governance completeness required by the constitution and spec.

- [ ] T006 [P] Create `changes/012-unify-ci-release.md` with frontmatter:
  `change_id: 012-unify-ci-release`, `release_intent: minor`,
  `summary: Unified CI validation and release publication into a single ci.yml workflow`,
  `scope: ci-release`, `release_preparation: false`

- [ ] T007 [P] Update `CHANGELOG.md` `[Unreleased]` section: add entry under `### Changed`
  describing the unified CI workflow (build matrix on every PR, release job on master push,
  live README badges, retirement of `release-binary.yml`)

- [ ] T008 Run `cargo test test_distribution_release` and confirm all tests pass

- [ ] T009 Run `cargo test test_distribution` and confirm full distribution suite passes

- [ ] T010 Run `cargo clippy --all-targets -- -D warnings` and confirm no new warnings

- [ ] T011 Validate `ci.yml` YAML syntax:
  `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`

- [ ] T012 Run `cargo run --bin core-ops-release -- validate --base-ref HEAD^` and confirm
  governance passes (fragment and CHANGELOG entry are present and consistent)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 3 (US1)**: No dependencies beyond existing `ci` job — can start immediately
- **Phase 4 (US2)**: Depends on T001 (build job must exist to be referenced by `needs: build`)
- **Phase 5 (US3)**: Depends on Phase 4 being functionally complete (T003, T004 parallel after T002)
- **Phase 6 (US4)**: Independent of other phases; can begin in parallel with Phase 3
- **Phase 7 (Polish)**: T006/T007 parallel; validation tasks after implementation complete

### User Story Dependencies

- **US1 (P1)**: No upstream story dependency
- **US2 (P2)**: Depends on US1 (`needs: build`)
- **US3 (P3)**: Depends on US2 being operational before deletion of `release-binary.yml`
- **US4 (P4)**: Fully independent; can be done any time after US1+US2 are merged

### Within Each Phase

- T001 before T002 (release job references `needs: build`)
- T002 before T003/T004 (retire workflow only once release job is in place)
- T003 and T004 are parallel (different files)
- T006 and T007 are parallel (different files)
- T008–T012 run after all implementation tasks complete

---

## Parallel Opportunities

```bash
# Phase 5 tasks run in parallel (different files):
T003: Delete .github/workflows/release-binary.yml
T004: Update tests/integration/test_distribution_release_artifacts.rs

# Phase 7 governance tasks run in parallel:
T006: Create changes/012-unify-ci-release.md
T007: Update CHANGELOG.md [Unreleased]
```

---

## Implementation Strategy

### MVP Scope (User Story 1 + 2 Only)

1. Complete T001 (build matrix + artifacts in ci.yml)
2. **Validate**: Push a test PR; confirm workflow artifacts appear
3. Complete T002 (release job in ci.yml)
4. **Validate**: Merge to master; confirm GitHub Release is created
5. **STOP and VALIDATE**: US1 + US2 are independently functional

### Full Delivery

1. T001 → validate US1 (PR artifacts)
2. T002 → validate US2 (GitHub Release)
3. T003 + T004 (parallel) → validate US3 (no release-binary.yml, tests pass)
4. T005 → validate US4 (live badges in README)
5. T006 + T007 (parallel) + T008–T012 → release governance complete

---

## Notes

- [P] tasks operate on different files with no mutual dependencies
- No new Rust source — `cargo clippy` validates only modified test files
- `release-binary.yml` must not be deleted (T003) until T002 (release job) is
  validated on master — otherwise there is no release publication path
- Live badges (T005) are cosmetic and can be done any time, but have most impact
  after the first GitHub Release is published via the new release job
- The `release_identity` in `release-metadata.json` changes format: was
  `${{ github.event.release.tag_name }}` (e.g. `v0.7.0`), now derived from
  `grep '^version' Cargo.toml` at build time — this is intentional and tested by T004
