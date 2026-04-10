# Implementation Plan: SemVer and Changelog Governance

**Branch**: `011-semver-changelog-governance` | **Date**: 2026-04-10 | **Spec**: [spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/011-semver-changelog-governance/spec.md)
**Input**: Feature specification from `/specs/011-semver-changelog-governance/spec.md`

**Note**: This plan covers Phase 0 research and Phase 1 design only.

## Summary

Enforce release governance for releasable pull requests by introducing
checked-in release fragments, deterministic release classification and SemVer
evaluation rules, generated changelog assembly, and a dedicated
`core-ops-release` helper binary that drives the required CI validation path
and rejects incomplete or misclassified release metadata.

## Technical Context

**Language/Version**: Rust 2021 (stable toolchain) for validation logic; Markdown and GitHub Actions workflow definitions for repo-facing governance surfaces  
**Primary Dependencies**: Existing CoreOps Rust stack (`clap`, `serde_json`, `miette`, `thiserror`, `tempfile`) plus standard library filesystem and process facilities; no new dependency is assumed in Phase 0/1  
**Storage**: Files on disk in the repository (`Cargo.toml`, `CHANGELOG.md`, release fragment files, workflow/test fixtures, specification artifacts)  
**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`, integration tests for workflow and governance contracts  
**Target Platform**: Linux development hosts and GitHub Actions pull request workflows for repository validation  
**Project Type**: Rust repository-governance automation with a dedicated helper binary in a single project  
**Performance Goals**: Governance validation completes within normal pull-request CI latency and remains deterministic for typical repository-sized diffs  
**Constraints**: Must preserve `Cargo.toml` as canonical controller version, keep release intent machine-checkable in-tree, generate `CHANGELOG.md` from fragments, and reject unspecified exemptions  
**Scale/Scope**: Single-repository governance for all pull requests targeting the default branch, covering code, accepted verification corpus, release workflows, and shipped support materials

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; side effects are isolated.
- Desired/observed state, reconciliation plans, and outcomes are represented as data.
- Abstractions are minimal and justified; complexity tracking added if needed.
- Effects, assumptions, and failure modes are explicit in interfaces and returns.
- Idempotence and convergence strategy are defined, including retry behavior.
- Open standards and native interfaces are preferred; deviations justified.
- Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs.
- Provenance and status surfaces identify reconciler revision, desired-state revision,
  and applied outcome in machine-readable form.
- Safe defaults are documented; destructive actions require explicit intent.
- Compatibility impact is assessed; breaking changes are documented with migration.
- Release version policy impact is assessed for any externally observable,
  schema, CLI, reconciliation, or compatibility change; the canonical
  controller version comes from `Cargo.toml`.
- Release intent is explicitly classified as `patch`, `minor`, or `major` for
  any releasable change, and the highest applicable bump is recorded.
- Changelog impact is assessed for any externally visible change, and
  releasable work is not considered complete until the Keep a
  Changelog-formatted changelog and machine-checkable release-intent artifact
  are updated alongside `Cargo.toml`.
- Rust changes include the required validation gate plan: `cargo test` and
  `cargo clippy --all-targets -- -D warnings`, or an explicit temporary
  exception with follow-up.
- Test strategy covers invariants, external behavior, convergence, and failures.
- Modules are structured to be regenerable from specs and tests.

**Gate Result (Pre-Research)**: PASS. The feature remains data-oriented, keeps
release intent in versioned repository files, uses explicit classification
rules, and fits existing Rust + workflow validation boundaries without
constitutional violations.

## Project Structure

### Documentation (this feature)

```text
specs/011-semver-changelog-governance/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── release-fragment-contract.md
│   └── release-governance-check-contract.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
├── bin/
├── core/
├── io/
└── lib.rs

tests/
├── integration/
└── unit/

.github/workflows/
├── ci.yml
├── e2e-gate.yml
└── release-binary.yml

CHANGELOG.md
Cargo.toml
AGENTS.md
```

**Structure Decision**: Single Rust project with shared repository-governance
logic in existing core/io boundaries, a dedicated `src/bin/core-ops-release.rs`
helper binary as the contributor-facing execution surface, plus contract tests
under `tests/integration/` and repository artifacts at root and
`.github/workflows/`.

## Phase 0: Research

- Confirm the release-intent artifact strategy as checked-in release fragments.
- Confirm changelog generation strategy from approved fragments rather than
  per-PR direct editing.
- Confirm release classification policy split into always exempt, always
  releasable, and context-dependent rules.
- Confirm SemVer decision table and metadata-only release-preparation policy.

## Phase 1: Design & Contracts

- Model release fragments, classification rules, exemption rules, governance
  evaluation results, and release-preparation changes.
- Define contract surfaces for:
  - checked-in release fragment content
  - governance-check output and failure reporting expectations
- Define contributor quickstart for adding a releasable change, fragment, and
  validating CI behavior locally.
- Update agent context to reflect this feature’s governance and tooling scope.

## Post-Design Constitution Check

**Gate Result (Post-Design)**: PASS. Phase 1 design keeps classification and
SemVer evaluation deterministic, makes failures explicit and machine-readable,
preserves `Cargo.toml` as canonical version state, and encodes release intent
as in-repo artifacts suitable for regeneration and CI enforcement.

## Complexity Tracking

No constitutional violations or justified complexity exceptions identified in
Phase 0/1 planning.
