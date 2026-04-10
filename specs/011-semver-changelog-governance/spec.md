# Feature Specification: SemVer and Changelog Governance

**Feature Branch**: `011-semver-changelog-governance`  
**Created**: 2026-04-10  
**Status**: Draft  
**Input**: User description: "Use `.agent/spec.md` as input."

## Clarifications

### Session 2026-04-10

- Q: Where should machine-checkable release intent live? → A: One checked-in release fragment file per PR/change records release intent.
- Q: How should `CHANGELOG.md` be maintained? → A: `CHANGELOG.md` is generated from approved checked-in release fragments.
- Q: How should workflow-only changes be classified? → A: Workflow changes are releasable only when they affect release, verification, or operator-facing behavior.
- Q: How should accepted verification corpus changes be classified? → A: Accepted verification corpus changes always require at least a patch bump.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reject Incomplete Release Metadata (Priority: P1)

As a maintainer, I want every releasable pull request to include an intentional
version bump, changelog update, and release intent declaration so that trunk
stays releasable and merges cannot silently ship undocumented behavior.

**Why this priority**: This is the core governance outcome. Without it, the
repository can still accept unreleasable changes and automated release flows
remain unsafe.

**Independent Test**: Open a pull request that changes shipped behavior without
updating release-governance artifacts and verify that the required governance
check fails with a clear reason.

**Acceptance Scenarios**:

1. **Given** a pull request that changes shipped behavior, **When** the pull
   request omits the required version update, **Then** the release-governance
   check fails before merge.
2. **Given** a pull request that changes shipped behavior, **When** the pull
   request omits the changelog update or release intent declaration, **Then**
   the release-governance check fails before merge.
3. **Given** a pull request that changes releasable files and exempt files,
   **When** release metadata is missing, **Then** the governance check fails
   and reports that exempt files do not override releasable deltas.

---

### User Story 2 - Enforce Correct SemVer Intent (Priority: P2)

As a maintainer, I want pull requests to declare the correct Semantic
Versioning impact so that release numbers communicate the real scope of change.

**Why this priority**: Rejecting missing metadata is not enough if the declared
version bump can still be wrong. The release number must remain trustworthy.

**Independent Test**: Open pull requests that declare `patch`, `minor`, and
`major` release intent for representative changes and verify that the required
check passes only when the declared bump matches the required impact.

**Acceptance Scenarios**:

1. **Given** a pull request with an additive backward-compatible change,
   **When** it declares only a patch bump, **Then** the release-governance
   check fails and identifies the mismatch.
2. **Given** a pull request with mixed patch-level and minor-level changes,
   **When** it declares the highest applicable bump, **Then** the
   release-governance check passes.
3. **Given** a pull request with a breaking change and an additive change,
   **When** it declares `minor`, **Then** the governance check fails and
   requires `major`.

---

### User Story 3 - Preserve Fast Paths for Exempt Changes (Priority: P3)

As a contributor, I want clearly exempt non-release-affecting changes to avoid
unnecessary release metadata updates so that routine documentation and
non-functional edits stay lightweight.

**Why this priority**: Governance must remain strict for releasable work
without creating friction for changes that do not affect what is shipped.

**Independent Test**: Open a documentation-only or formatting-only pull request
and verify that the governance check passes without requiring release metadata
changes.

**Acceptance Scenarios**:

1. **Given** a pull request that changes only exempt files, **When** the
   governance check runs, **Then** it passes without requiring a version bump
   or changelog update.
2. **Given** a pull request that changes only version or changelog metadata,
   **When** no checked-in fragment declares `release_preparation: true`,
   **Then** the governance check fails.
3. **Given** a pull request that changes only version or changelog metadata,
   **When** the checked-in fragment declares `release_preparation: true`,
   **Then** the governance check passes if all other release-governance rules
   are satisfied.

### Edge Cases

- A pull request changes both releasable and exempt files; the governance check
  must still require full release metadata.
- A pull request updates release metadata but does not change shipped behavior;
  the governance check must fail unless the pull request is explicitly marked
  as release-preparation work by the machine-checkable release-intent model.
- Multiple releasable changes in one pull request require different bump
  levels; the governance check must enforce the highest applicable bump.
- A pull request updates version and changelog but omits the machine-checkable
  release intent declaration; the governance check must fail rather than infer
  intent implicitly.

## Release Classification Policy

### Always Exempt

The following changes are exempt unless combined with releasable deltas in the
same pull request:

- documentation-only content that is not shipped as part of the release bundle
- comments-only edits
- formatting-only edits
- internal workflow text or metadata edits that do not affect release,
  verification, or operator-facing behavior

### Always Releasable

The following changes are always releasable:

- Rust source changes affecting shipped binaries or verification tooling
- changes to accepted verification corpus entries
- changes to public CLI contracts, machine-readable output contracts, API or
  schema contracts, default configuration behavior, release workflows, release
  metadata, packaged documentation, or installation/support promises
- workflow changes that affect release behavior, verification guarantees, or
  operator-facing behavior

### Context-Dependent

The following changes require semantic evaluation against their effect:

- tests, fixtures, examples, and generated artifacts
- configuration files or schema files not directly exposed to users
- workflow-only changes outside the protected release and verification paths

Context-dependent items are treated as releasable when they change public
behavior, release guarantees, accepted verification claims, or operator-facing
outcomes. They are exempt otherwise.

When a pull request contains both exempt and releasable changes, the whole pull
request is treated as releasable.

## SemVer Classification Policy

### Patch

Use `patch` for:

- bug fixes without contract expansion
- internal corrections that preserve existing supported behavior
- non-breaking output corrections
- accepted verification corpus updates that tighten or repair existing contract
  coverage without adding new supported capability

### Minor

Use `minor` for:

- backward-compatible new capability
- additive CLI, API, schema, config, or workflow surface
- new supported deployment or verification pattern without breaking existing
  behavior

### Major

Use `major` for:

- removed, renamed, or incompatible behavior
- changed machine-readable output contracts
- incompatible default changes
- tighter validation or schema rules that reject previously valid inputs
- changes requiring operator migration or release-consumer adaptation

### Highest-Bump Rule

If multiple changes in one pull request require different bump levels, the
effective release intent MUST be the highest applicable bump.

## Release Metadata Workflow

- Every releasable pull request MUST bump the canonical version in
  `Cargo.toml` immediately.
- Every releasable pull request MUST add or update exactly one checked-in
  release fragment at `changes/<change-id>.md` that declares SemVer intent and
  release-note content.
- Release-preparation work is represented only by `release_preparation: true`
  in the checked-in release fragment.
- `CHANGELOG.md` is generated from approved checked-in release fragments and is
  not the source of truth for per-PR intent.
- Generated `CHANGELOG.md` output MUST be updated in the same releasable change
  set as the `Cargo.toml` and release fragment updates it reflects.
- Metadata-only version or changelog changes are rejected unless the pull
  request is explicitly designated as release-preparation work by the
  machine-checkable release-intent model.
- The governance check is enforced on pull requests targeting the default
  branch.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST determine whether a pull request contains a
  releasable change or only an explicitly exempt change set using the Release
  Classification Policy in this specification.
- **FR-002**: The system MUST require every releasable pull request to update
  the canonical project version in `Cargo.toml`.
- **FR-003**: The system MUST require every releasable pull request to update
  user-relevant release-note content in a checked-in release fragment that is
  used to generate `CHANGELOG.md` in the same releasable change set.
- **FR-004**: The system MUST require every releasable pull request to include
  a machine-checkable release intent declaration stored in a checked-in release
  fragment file at `changes/<change-id>.md`.
- **FR-005**: The system MUST classify release intent using Semantic
  Versioning levels `patch`, `minor`, and `major` according to the SemVer
  Classification Policy in this specification.
- **FR-006**: The system MUST validate that the effective version bump matches
  the declared release intent for the pull request.
- **FR-007**: The system MUST apply the highest applicable SemVer bump when a
  pull request includes multiple releasable changes with different impact
  levels.
- **FR-008**: The system MUST fail the required governance check when a
  releasable pull request is missing any required release-governance artifact.
- **FR-009**: The system MUST fail the required governance check when the
  declared release intent does not match the required version bump.
- **FR-010**: The system MUST maintain explicit, machine-checkable exemption
  rules for non-release-affecting changes aligned with the Release
  Classification Policy in this specification.
- **FR-010a**: Workflow-only changes MUST be treated as releasable when they
  affect release behavior, verification guarantees, or operator-facing
  behavior, and MAY be exempt otherwise.
- **FR-010b**: Changes to the accepted verification corpus MUST require at
  least a patch bump, even when no shipped Rust code changes in the same pull
  request.
- **FR-011**: The system MUST allow exempt pull requests to pass without
  version, changelog, or release-intent updates.
- **FR-014**: The system MUST generate `CHANGELOG.md` from approved checked-in
  release fragments rather than requiring direct manual changelog edits in each
  releasable pull request.
- **FR-012**: Repository-level agent guidance MUST define when release
  metadata is required and where release fragments must be placed so that
  agent-authored pull requests can satisfy the governance check deterministically.
- **FR-013**: The governance check MUST be suitable for use as a required pull
  request status check.
- **FR-015**: The system MUST reject metadata-only version or changelog changes
  unless the pull request is explicitly designated as release-preparation work
  by `release_preparation: true` in the machine-checkable release fragment.

### Key Entities *(include if feature involves data)*

- **Release Intent Declaration**: The machine-checkable artifact that states
  whether a pull request is `patch`, `minor`, or `major`, stored as a checked-
  in release fragment file.
- **Release Fragment**: A checked-in per-change artifact that carries SemVer
  intent and human-readable release-note content used to generate
  `CHANGELOG.md`.
- **Release Classification Rule**: A policy rule that determines whether a
  change is releasable or exempt and what SemVer bump it requires.
- **Release Governance Check**: The pull request validation outcome that
  accepts or rejects the change set based on version, changelog, and release
  intent completeness.
- **Exemption Rule**: A machine-checkable rule defining which change classes
  are allowed to bypass release metadata requirements.
- **Release-Preparation Change**: A pull request intentionally dedicated to
  versioning or release-note preparation and explicitly declared as such by the
  release-intent model.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Release classification and validation
  should be expressed as deterministic evaluation over changed-file and
  metadata inputs, with CI integration isolated to the workflow boundary.
- **Declarative state model**: Pull request deltas, version state, changelog
  state, release intent, and exemption rules must be represented explicitly as
  data that can be evaluated and reported.
- **Idempotence & convergence**: Re-running the governance check on unchanged
  pull request contents must produce the same outcome and diagnostics.
- **Explicit effects/failures**: Governance failures must identify which
  required artifact or SemVer rule is missing or inconsistent.
- **Observability**: The check output must explain why a pull request passed or
  failed and identify the required release-governance updates.
- **Provenance & traceability**: The feature must preserve the canonical
  controller version in `Cargo.toml` and make the declared release intent
  auditable at review time.
- **Safe defaults**: Releasable changes default to requiring explicit release
  metadata; exemptions must be explicit rather than inferred loosely.
- **Compatibility**: The specification preserves backward-compatible release
  numbering semantics while making breaking changes impossible to merge without
  intentional version review.
- **Release version policy**: This feature formalizes how behavior,
  compatibility, and other releasable impacts map to SemVer bumps anchored in
  `Cargo.toml`.
- **Release intent artifact**: The implementation must define a
  machine-checkable release fragment file that CI can evaluate without relying
  on human interpretation.
- **Changelog discipline**: Releasable work is incomplete until
  generated `CHANGELOG.md`, the declared version, and the validated release
  intent remain aligned.
- **Test contract**: The feature requires deterministic automated checks for
  missing metadata, incorrect bump selection, and exempt change behavior.
- **Regenerability**: Release-governance rules should be encoded so agents and
  future tooling can regenerate compliant changes from the spec and tests.

## Assumptions

- CoreOps will continue to use `Cargo.toml` as the canonical version source for
  shipped binaries.
- The implementation will introduce exactly one machine-checkable
  release-intent mechanism using checked-in release fragment files.
- `CHANGELOG.md` will remain a published human-readable artifact, but its
  content will be generated from approved release fragments rather than edited
  directly in each releasable pull request.
- Governance enforcement will run on pull requests before merge and will be
  configured as a required status check on pull requests to the default branch.
- Exemption rules will remain narrow and focused on clearly non-releasable
  edits such as documentation, comments, formatting-only changes, and
  workflow-only changes that do not affect release, verification, or
  operator-facing behavior.
- Accepted verification corpus changes are treated as release-affecting because
  they change the project's asserted behavioral contract and release
  credibility surface.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of pull requests containing releasable changes are blocked
  from merge when version, changelog, or release-intent updates are missing.
- **SC-002**: 100% of validated releasable pull requests produce a version bump
  that matches the declared SemVer impact.
- **SC-003**: 100% of exempt-only pull requests pass the governance check
  without requiring release metadata updates.
- **SC-004**: Reviewers can determine the intended release impact of a pull
  request from repository artifacts and CI results without needing private or
  out-of-band explanation.
