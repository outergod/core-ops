# Implementation Plan: Distribution Readiness

**Branch**: `010-distribution-readiness` | **Date**: 2026-04-08 | **Spec**: [/home/outergod/code/github.com/outergod/core-ops/specs/010-distribution-readiness/spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/010-distribution-readiness/spec.md)
**Input**: Feature specification from `/specs/010-distribution-readiness/spec.md`

## Summary

Prepare CoreOps for first outside consumption by adding a stable public
entrypoint, binary-only multi-architecture release surfaces for `x86_64` and
`aarch64`, explicit license and community documents, reproducible release-gate
automation, visible version/provenance surfaces, and a documented cold-start
install-and-verify flow for Fedora CoreOS that also includes the canonical
`core-ops.service` and `core-ops.timer` unattended execution path. The
implementation will extend existing CLI/report/version surfaces and
verification artifacts while introducing public documentation, changelog,
release metadata, and CI/CD workflow definitions as explicit, testable
contracts.

## Technical Context

**Language/Version**: Rust 2021 for shipped binaries and verification tooling; Markdown/YAML/shell for public docs and automation definitions  
**Primary Dependencies**: Existing CoreOps Rust stack (`clap`, `miette`, `thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `systemd-journal-logger`, `tempfile`, `time`), git metadata from the repository, curl-consumable binary distribution surfaces, forge-hosted automation definitions  
**Storage**: Files on disk for public documentation, changelog, license/code-of-conduct documents, workflow definitions, release metadata, and existing verification artifacts  
**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`, integration tests for public output/report surfaces, protected authoritative E2E gate execution on Fedora CoreOS, and documentation/contract validation for release materials  
**Target Platform**: Linux hosts; officially supported on Fedora CoreOS, theoretically compatible but untested on other systemd-based hosts, unsupported on non-systemd environments  
**Project Type**: CLI tool plus release automation and public distribution documentation  
**Performance Goals**: No material regression to normal CLI or verification-report responsiveness; release-gate checks remain deterministic and bounded by the accepted verification corpus runtime  
**Constraints**: Binary-only distribution for this feature, published binary support must cover `x86_64` and `aarch64`, the outside-consumption story must include the canonical `core-ops.service` and `core-ops.timer` host integration path, AGPLv3+ licensing, code of conduct required, credibility surface must remain stably locatable in the project entrypoint, installation and verification flows must pass from a fresh supported environment, containerized CoreOps execution must remain explicitly unsupported, authoritative verification environment must be documented/reproducible/versioned  
**Scale/Scope**: Public entrypoint and release-material work across repository-root documentation, release metadata, workflow definitions, build/version surfaces, and tests; no new runtime configuration model or reconciliation semantics

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; policy and public-surface rules stay data-oriented while release publishing, workflow execution, and artifact generation remain boundary work.
- Desired/observed state, reconciliation plans, and outcomes remain represented as data; this feature adds release identity, credibility, and support-boundary contracts rather than hidden runtime behavior.
- Abstractions are minimal and justified; most changes land in existing documentation, CLI report/version surfaces, and workflow definitions instead of new subsystems.
- Effects, assumptions, and failure modes are explicit in interfaces and returns, especially for release-gate failure, environment drift, and public-surface mismatch.
- Idempotence and convergence strategy are defined: repeated release-gate execution on the same candidate/environment must yield the same decision; repeated install/verify flows on fresh supported systems must not require hidden setup.
- Open standards and native interfaces are preferred: Markdown docs, changelog conventions, AGPLv3+, binary downloads, git/version metadata, and existing verification infrastructure.
- Observability plan covers public credibility signals, release-gate outcomes, version/provenance visibility, changelog continuity, and operator-facing failure surfaces.
- Provenance and status surfaces will continue to identify controller version, source revision, spec context, release-gate state, and verification-environment identity in machine-readable and humane outputs.
- Safe defaults are documented; unsupported environments and containerized execution are explicitly excluded instead of implied safe.
- Compatibility impact is assessed; README/entrypoint, changelog, release identity surfaces, and version-reporting are public contracts and must evolve conservatively.
- Release version policy impact is assessed; this feature changes external docs, version/provenance surfaces, distribution artifacts, and release automation, so release-policy review is mandatory.
- Rust changes include the required validation gate plan: `cargo test` and `cargo clippy --all-targets -- -D warnings`.
- Test strategy covers public invariants, install/verify flow behavior, release-gate determinism, version/provenance visibility, and public failure semantics.
- Modules remain regenerable from specs and tests because release requirements are captured as explicit contracts rather than ad hoc checklist knowledge.

## Project Structure

### Documentation (this feature)

```text
specs/010-distribution-readiness/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
└── tasks.md
```

### Source Code (repository root)

```text
README.md
CHANGELOG.md
LICENSE
CODE_OF_CONDUCT.md
.github/workflows/
docs/
├── development.md
└── distribution-readiness-proposal.md
src/
├── build_info.rs
├── main.rs
├── bin/core-ops-verify.rs
├── cli/
│   ├── args.rs
│   ├── report.rs
│   ├── explain.rs
│   ├── status.rs
│   ├── verification.rs
│   └── diagnostics.rs
├── core/
│   ├── verification_model.rs
│   ├── verification_eval.rs
│   └── verification_generate.rs
└── io/
    └── verification_artifacts.rs
tests/
├── integration/
│   ├── test_apply_report.rs
│   ├── test_status_contract.rs
│   ├── test_verification_cli.rs
│   ├── test_verification_execution.rs
│   └── test_quickstart_verification.rs
└── unit/
    ├── test_verification_execution.rs
    └── test_verification_model.rs
tests/fixtures/verification/
```

**Structure Decision**: Keep the existing single Rust CLI project structure.
Add public distribution documents at repository root, introduce automation under
`.github/workflows/`, and extend existing CLI/report/verification modules and
tests rather than creating a separate release tool.

## Phase 0: Research

- Determine the minimal binary-only distribution shape that satisfies the spec
  without prematurely committing to RPM packaging.
- Define a stable public credibility surface and what signals belong there.
- Define the authoritative verification-environment identity contract needed to
  detect self-hosted runner drift.
- Define the operator-facing install, smoke-test, and verification flow so it
  is cold-start safe on Fedora CoreOS.
- Define how AGPLv3+, code of conduct, changelog, version/provenance, and AI
  authorship disclosure should appear in public materials without ambiguity.

## Phase 1: Design & Contracts

- Model release identity, credibility, distribution, installation, operator
  verification, and verification-environment identity as explicit data/contract
  objects.
- Define public contracts for:
  - project entrypoint structure and required public signals
  - release-gate decision and environment identity
  - operator install-and-verify flow expectations
- Document a quickstart that exercises the binary-only install path and the
  minimal operator verification flow on a fresh supported environment.

## Post-Design Constitution Check

- Functional core vs side effects remains preserved by keeping release rules in
  explicit data/contracts and isolating artifact publication/workflow execution
  to boundaries.
- Public observability is strengthened rather than weakened: credibility,
  changelog, version, provenance, and release-gate outcomes all become more
  inspectable.
- Compatibility-sensitive surfaces are explicitly enumerated so future changes
  can update release policy intentionally.
- No constitution violations are expected from the planned design.

## Complexity Tracking

No constitution exceptions are expected for this feature.
