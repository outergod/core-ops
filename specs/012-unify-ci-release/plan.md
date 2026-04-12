# Implementation Plan: Unify CI Validation And Release Publication

**Branch**: `012-unify-ci-release` | **Date**: 2026-04-11 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/012-unify-ci-release/spec.md`

## Summary

Consolidate CI validation and release publication into a single `ci.yml` workflow.
The `ci` job (test/clippy/governance) gates a cross-compilation `build` matrix that uploads
workflow artifacts on every PR. A `release` job — triggered only on push to `master`, using
the default `GITHUB_TOKEN` — derives the version from `Cargo.toml`, checks for duplicate tags,
extracts release notes from `CHANGELOG.md`, and publishes a GitHub Release with attached binary
assets. The separate `release-binary.yml` is retired. Distribution integration tests are updated
to assert the unified workflow contract. The README Credibility section is replaced with live badges.

## Technical Context

**Language/Version**: Rust 2021 (no new Rust source); GitHub Actions YAML  
**Primary Dependencies**: `actions/checkout@v4`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`, `gh` CLI (pre-installed on `ubuntu-latest`), `cargo`, `rustup`  
**Storage**: N/A  
**Testing**: `cargo test` + `cargo clippy --all-targets -- -D warnings`; distribution integration tests in `tests/integration/test_distribution_release_artifacts.rs`  
**Target Platform**: GitHub Actions (ubuntu-latest runners)  
**Project Type**: CI workflow infrastructure change (no new runtime code)  
**Performance Goals**: N/A  
**Constraints**: No PAT; default `GITHUB_TOKEN` only; release job scoped to master push  
**Scale/Scope**: Single repository; two cross-compilation targets (x86_64, aarch64)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] Functional core and imperative shell boundaries are explicit; side effects are isolated.
  - Version reading, changelog extraction: pure reads. Tag creation, release publication, asset upload: explicit side effects in the `release` job only, gated to master push.
- [x] Desired/observed state, reconciliation plans, and outcomes are represented as data.
  - Desired release state is declared in `Cargo.toml` (version) and `CHANGELOG.md`. The release job converges GitHub Releases toward that state.
- [x] Abstractions are minimal and justified; complexity tracking added if needed.
  - No new abstractions. Three jobs in a linear dependency chain.
- [x] Effects, assumptions, and failure modes are explicit in interfaces and returns.
  - FR-005: explicit failure on duplicate tag. FR-011: explicit CHANGELOG extraction. All failure modes documented in data-model.md.
- [x] Idempotence and convergence strategy are defined, including retry behavior.
  - New version: idempotent on retry (after orphaned-tag cleanup). Duplicate version: explicit failure, operator recovery documented in quickstart.md.
- [x] Open standards and native interfaces are preferred; deviations justified.
  - GitHub Actions native, `gh` CLI (first-party GitHub tool), standard YAML. No third-party action dependencies added.
- [x] Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs.
  - PR artifacts visible in Actions UI. Release publication logged per step. README badges expose live status to outside evaluators.
- [x] Provenance and status surfaces identify reconciler revision, desired-state revision, and applied outcome.
  - GitHub Release links to commit SHA; release tag anchors version to specific history point. `release-metadata.json` carries `latest_release_identity`.
- [x] Safe defaults are documented; destructive actions require explicit intent.
  - Release job gated to master push only. Duplicate detection prevents accidental overwrite.
- [x] Compatibility impact is assessed; breaking changes are documented with migration.
  - Existing PR CI behavior unchanged. `release-binary.yml` triggers (manual `workflow_dispatch` + `release` event) are retired — no active consumer other than the defunct release chain.
- [x] Release version policy impact is assessed.
  - This is a `minor` change: modifies externally observable release behavior (publication method, README badges). Version bump from `0.7.0` to `0.7.0` (already dev) — a CHANGELOG entry under `[Unreleased]` is required.
- [x] Release intent is explicitly classified.
  - `minor` — new CI behavior affects the release publication contract (externally observable), README, and workflow structure.
- [x] Changelog impact is assessed.
  - `CHANGELOG.md` must be updated with a `[Unreleased]` entry (or version entry) covering unified workflow change.
- [x] Rust changes include the required validation gate plan.
  - No new Rust source. Existing `cargo test` + `cargo clippy` gates apply to updated integration tests.
- [x] Test strategy covers invariants, external behavior, convergence, and failures.
  - `test_distribution_release_artifacts.rs`: updated to assert unified `ci.yml` structure. New test function for release job contract. Existing gate split test remains valid.
- [x] Modules are structured to be regenerable from specs and tests.
  - Workflow YAML fully described in `contracts/ci-workflow.md`; test contract in `data-model.md`.

**No constitution violations. No Complexity Tracking entry required.**

## Project Structure

### Documentation (this feature)

```text
specs/012-unify-ci-release/
├── plan.md              ← this file
├── research.md          ← Phase 0 output
├── data-model.md        ← Phase 1 output
├── quickstart.md        ← Phase 1 output
├── contracts/
│   └── ci-workflow.md   ← Phase 1 output
└── tasks.md             ← Phase 2 output (speckit.tasks)
```

### Files Modified / Deleted (repository root)

```text
.github/workflows/
├── ci.yml                     ← REWRITE (add build matrix + release job)
└── release-binary.yml         ← DELETE (retired by FR-007)

tests/integration/
└── test_distribution_release_artifacts.rs  ← UPDATE (ci.yml assertions)

README.md                      ← UPDATE (Credibility section → live badges)
CHANGELOG.md                   ← UPDATE ([Unreleased] entry)
changes/
└── 012-unify-ci-release.md    ← NEW (release intent fragment)
```

No new source directories. No new Rust modules.

## Implementation Phases

### Phase 1 — Workflow Rewrite (core behavior, P1 + P2 + P3)

**Task 1.1** — Rewrite `.github/workflows/ci.yml`:
- Keep existing `ci` job content unchanged (build, test, clippy, governance)
- Add `build` job: matrix `[x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]`, needs `ci`, packaging steps from `release-binary.yml`, upload artifact
- Add `release` job: `if: github.ref == 'refs/heads/master' && github.event_name == 'push'`, needs `build`, `contents: write`, derive version, check duplicate tag, extract CHANGELOG notes, download artifacts, `gh release create`

**Task 1.2** — Delete `.github/workflows/release-binary.yml`

### Phase 2 — Test Updates (FR-008)

**Task 2.1** — Update `tests/integration/test_distribution_release_artifacts.rs`:
- Change `release_binary_workflow_includes_license_and_metadata_outputs` to read `ci.yml`
- Replace `release_identity=\"${{ github.event.release.tag_name }}\"` assertion with `grep '^version' Cargo.toml`
- Add new test `unified_release_job_is_gated_to_master_push` asserting: `refs/heads/master`, `git ls-remote --tags origin`, `gh release create`, `contents: write`

### Phase 3 — README Badge Migration (P4)

**Task 3.1** — Update `README.md` Credibility section:
- Replace static table rows for CI status, E2E gate status, and latest release version with live badges (GitHub-native for CI/E2E; shields.io for release version)
- Retain static rows where no live equivalent exists (Published artifacts, Verification environment)

### Phase 4 — Release Governance Artifacts

**Task 4.1** — Create `changes/012-unify-ci-release.md` (release intent fragment, `minor`)

**Task 4.2** — Update `CHANGELOG.md` `[Unreleased]` section with unified workflow entry

## Complexity Tracking

No constitution violations require justification.
