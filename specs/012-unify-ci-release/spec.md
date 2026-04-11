# Feature Specification: Unify CI Validation And Release Publication

**Feature Branch**: `012-unify-ci-release`  
**Created**: 2026-04-10  
**Status**: Draft  
**Input**: Unify CI Validation And Release Publication from docs/follow-ups.md; transform README Credibility section into live badges as a coherent downstream step.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - PR Validation With Build Artifacts (Priority: P1)

A contributor opens a pull request with releasable changes. They want automated feedback confirming their change meets release governance rules and that the binaries build successfully. The CI workflow runs the governance check, builds all distributed binaries, and uploads them as workflow artifacts accessible from the PR checks panel.

**Why this priority**: Governance feedback on PRs is the primary value of the existing `ci.yml` ownership of the `core-ops-release validate` step. Building and surfacing artifacts in PRs replaces the broken `release-binary.yml` trigger pattern without changing where that binary work happens.

**Independent Test**: Can be tested end-to-end by opening a PR with a releasable change, verifying the governance check passes, and downloading the uploaded workflow artifacts from the Actions UI.

**Acceptance Scenarios**:

1. **Given** a pull request with compliant release metadata, **When** the CI workflow runs, **Then** the governance check passes, binaries are built, and workflow artifacts are uploaded.
2. **Given** a pull request with missing release metadata for a releasable change, **When** the CI workflow runs, **Then** the governance check fails and the PR cannot be merged.
3. **Given** a pull request with only exempt changes (docs, workflow files), **When** the CI workflow runs, **Then** the governance check passes as exempt and binaries are still built.

---

### User Story 2 - Canonical Release On Trunk Push (Priority: P2)

A maintainer merges a validated pull request to `master`. They expect the canonical GitHub Release to appear automatically, using the version already committed in `Cargo.toml`, with the built binaries attached as release assets. No manual trigger, no separate workflow file, no GITHUB_TOKEN token-chain problem.

**Why this priority**: This is the core behavioral change — moving release publication off a separate workflow triggered by a GitHub Release event and onto a trusted `push`-to-`master` job inside the same `ci.yml`. The follow-up explicitly identifies the GITHUB_TOKEN chained-workflow behavior as a poor fit for the current model.

**Independent Test**: Can be tested by merging a compliant PR to master and verifying a GitHub Release with the correct version tag and binary assets appears without manual intervention.

**Acceptance Scenarios**:

1. **Given** a push to `master` with a version in `Cargo.toml` that does not yet have a corresponding release tag, **When** the release job runs, **Then** a `v<version>` tag is created, a GitHub Release is published, and all binary assets are attached.
2. **Given** a push to `master` where the version tag already exists (e.g., a non-release commit that does not bump the version), **When** the release job runs, **Then** the release job fails explicitly with a clear message explaining the duplicate version, and no silent overwrite occurs.
3. **Given** a push to a branch other than `master`, **When** CI runs, **Then** the release job does not execute.

---

### User Story 3 - Retire Separate Release Workflow (Priority: P3)

A contributor examining the repository's CI configuration expects a single authoritative workflow for both validation and publication. The separate `release-binary.yml` no longer exists; its responsibilities are consolidated in `ci.yml`.

**Why this priority**: Retiring the orphaned workflow is the cleanup step that completes the unification. It cannot precede P2 being operational.

**Independent Test**: Can be tested by confirming `release-binary.yml` is absent from the repository and that no live release depends on it.

**Acceptance Scenarios**:

1. **Given** the unified `ci.yml` is in place and operational, **When** `release-binary.yml` is deleted, **Then** no currently active release publication path is broken.
2. **Given** the repository state after deletion, **When** a contributor reads the workflow directory, **Then** only `ci.yml` and `e2e-gate.yml` remain, with `ci.yml` handling both validation and publication responsibilities.

---

### User Story 4 - Live Credibility Badges In README (Priority: P4)

An outside evaluator visits the README. Instead of static values for release identity and CI status, they see live badges reflecting actual current state: the latest published release version, the status of the CI workflow, and the status of the E2E gate. The static table is replaced with badges that update automatically as the project evolves.

**Why this priority**: The README itself already anticipates this transition ("Live badges can replace individual values after the default branch has established real CI, protected E2E runs, and published release artifacts"). This feature delivers exactly those prerequisites, making badge migration the natural completion step. It is P4 because it is cosmetic and depends on P1–P2 being operational.

**Independent Test**: Can be tested by inspecting the README on the default branch and confirming that CI-status and release-version badges render with live data sourced from the repository's own GitHub Actions and Releases.

**Acceptance Scenarios**:

1. **Given** the unified workflow is operational and has produced at least one GitHub Release, **When** an evaluator views the README, **Then** the Credibility section shows live badge values for CI status, E2E gate status, and latest release version.
2. **Given** the CI workflow is currently failing, **When** an evaluator views the README, **Then** the CI status badge reflects the failure state, not a stale static string.
3. **Given** a new release is published, **When** an evaluator views the README, **Then** the latest-release badge updates to the new version without any manual README edit.

---

### Edge Cases

- What happens when the release job has no permission to create tags? The job must fail with a clear error message, not silently succeed.
- What happens when a push to master contains multiple commits with different Cargo.toml versions? The version at `HEAD` of the push is authoritative; the release job reads `Cargo.toml` at the pushed ref.
- What happens when the binary build fails but governance passes? The release job must not run; binary artifact availability is a prerequisite for publication.
- What badge service or URL format is used? Implementation choice; the spec requires the badges to be live-sourced, not the specific provider or URL.
- What happens if the Credibility table contains values that have no live-badge equivalent (e.g., published artifact list, verification environment string)? Those values may remain as static text alongside badges where live data is not available.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The CI workflow MUST run `core-ops-release validate` (or equivalent governance check) on every pull request targeting `master`.
- **FR-002**: The CI workflow MUST build all distributed binaries (`core-ops`, `core-ops-verify`, `core-ops-release`) on every pull request and upload them as workflow artifacts.
- **FR-003**: The CI workflow MUST include a release job that runs exclusively on `push` to `refs/heads/master` and is skipped for all other refs.
- **FR-004**: The release job MUST read the canonical version from `Cargo.toml` at the pushed commit and use it to derive the release tag (`v<version>`).
- **FR-005**: The release job MUST fail explicitly when a release or tag for the derived version already exists, and MUST NOT silently overwrite or skip without explanation.
- **FR-006**: The release job MUST attach the built binary assets from the same workflow run to the published GitHub Release.
- **FR-007**: The separate `release-binary.yml` workflow MUST be removed once the unified release job in `ci.yml` is operational.
- **FR-008**: Distribution-readiness fixtures and integration tests that assert workflow structure MUST be updated to reflect the unified workflow contract, not the separate `release-binary.yml` shape.
- **FR-009**: The README Credibility section MUST be updated to display live badges for at minimum: CI workflow status, E2E gate status, and latest published release version.
- **FR-010**: The README Credibility section badge replacement MUST occur only after the unified release job is operational and at least one GitHub Release has been published through it.

### Key Entities

- **Workflow artifact**: A binary file produced by a CI run and stored temporarily in GitHub Actions storage, accessible from the PR checks panel. Not a permanent release asset.
- **Release asset**: A binary file permanently attached to a GitHub Release. Produced by the release job from the same workflow run's build outputs.
- **Release tag**: A Git tag of the form `v<version>` derived from `Cargo.toml`. Canonical identity for a published release.
- **Duplicate version**: A state where the derived release tag already exists. Must be treated as an explicit failure, not a no-op.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Governance validation and version derivation are pure reads; tag creation, release publication, and asset upload are explicit side effects scoped to the trusted `push`-to-`master` path only.
- **Declarative state model**: The desired release state is declared in `Cargo.toml` (version) and `CHANGELOG.md` (changelog entry). The release job converges GitHub Releases toward that declared state.
- **Idempotence & convergence**: A push to master with an already-published version MUST fail rather than silently converge — duplicate publication is an integrity violation. A push with a new version is idempotent from the second attempt onward only if the first run partially failed before tagging; the spec requires explicit failure on duplicate, leaving recovery to operator judgment.
- **Explicit effects/failures**: Every release publication action (tag creation, release creation, asset upload) must be observable in the workflow log. Failures must produce a clear, actionable error. Silent skips are prohibited.
- **Observability**: PRs expose governance check results and downloadable workflow artifacts. Merged pushes expose a GitHub Release with version, changelog entry, and binary assets. README badges expose live CI and release status to outside evaluators.
- **Provenance & traceability**: The GitHub Release must link to the commit SHA that produced it. The release tag anchors the published version to a specific point in history.
- **Safe defaults**: The release job is gated to `push` to `master` only; no PR run can trigger publication. Duplicate version detection prevents accidental overwrite.
- **Compatibility**: PRs that currently pass `ci.yml` governance must continue to pass after this change. No behavioral change to governance validation logic itself.
- **Release version policy**: This feature is a minor change to CI workflow structure; the version bump for merging this work follows standard governance fragment rules.
- **Release intent artifact**: A `changes/<change-id>.md` fragment with appropriate `release_intent` and version bump declaration is required when merging this work.
- **Changelog discipline**: `CHANGELOG.md` must be updated with an entry covering the unified workflow change before the merge is considered complete.
- **Test contract**: Distribution-readiness fixtures that verify workflow structure must be updated. No new Rust logic is introduced that requires `cargo test` or `cargo clippy` gates beyond the existing CI baseline.
- **Regenerability**: The spec and updated fixtures fully describe the expected workflow contract, enabling safe future regeneration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every pull request receives automated governance validation feedback without manual intervention.
- **SC-002**: Binary workflow artifacts are available for download from every pull request CI run within the normal CI completion window.
- **SC-003**: A push to master with a new version produces a published GitHub Release with attached binary assets without any manual step by a maintainer.
- **SC-004**: A push to master with an existing version tag produces a visible CI failure within the release job with a message identifying the duplicate, rather than silently succeeding or skipping.
- **SC-005**: The `release-binary.yml` file is absent from the repository's default branch workflow directory.
- **SC-006**: The README Credibility section contains at least three live badges (CI status, E2E gate status, latest release version) that update automatically without manual README edits when CI status or release version changes.
- **SC-007**: An outside evaluator reading the README Credibility section can determine the current live CI health and latest release version without navigating away from the README.
