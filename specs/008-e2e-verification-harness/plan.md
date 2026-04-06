# Implementation Plan: E2E Verification Harness with LLM-Assisted Scenario Generation

**Branch**: `008-e2e-verification-harness` | **Date**: 2026-04-01 | **Spec**: [spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/spec.md)
**Input**: Feature specification from `/specs/008-e2e-verification-harness/spec.md`

## Summary

Add a libvirt-backed end-to-end verification harness to CoreOps that executes
declarative, single-VM scenarios against disposable guests, preserves
diagnostic artifacts for every run, and emits deterministic human and
machine-readable results for local, CI, and debug workflows. The implementation
should keep scenario modeling, validation, taxonomy checks, and result
classification pure, while isolating provisioning, guest command execution,
artifact collection, and candidate generation behind explicit boundary layers.
VM-backed disposable-machine execution is the authoritative verification mode
for the feature. Any synthetic or non-VM backend exists only to support
deterministic internal automated validation and does not satisfy the product
goal on its own.
The verification system must cover scenario modeling, repository evolution
sequences, runtime harness execution, command and output contract verification,
scenario generation, accepted regression corpus management, developer and CI
workflows, and coarse operational timing guardrails.
The scenario contract should favor authorability in the common case by
separating behavioral intent, environment profile selection, and harness-policy
overrides, with reusable defaults and structured semantic actions instead of
fully explicit harness configuration for every accepted scenario.
Existing UAT assets in `justfile`, `infra/ignition`, and `docs/development.md`
(`VM Host Preparation (CoreOS)`) are part of the research baseline and should
be evaluated for reuse, extension, or migration during implementation.
Phase 2 resolves that decision early: v1 will reuse `just render-ignition`
and the existing `infra/ignition` templates as the initial guest-bootstrap
path behind the harness boundary, while keeping manual UAT eligible to migrate
onto any cleaner replacement path the harness later adopts. Verification
execution remains on a dedicated development or testing entrypoint rather than
expanding the stable operator-facing `core-ops` binary surface.
Scenario derivation remains spec-driven: the feature specification is the
canonical semantic input for candidate generation, while any structured
verification guidance in the spec is optional support for LLM prompting,
normalization quality, and reviewer context rather than a required replacement
for semantic reading of the spec.

## Technical Context

**Language/Version**: Rust 2021  
**Primary Dependencies**: Existing CoreOps Rust stack (`clap`, `miette`,
`thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `tempfile`),
native libvirt tooling (`virsh`, `qemu-img`), SSH client tooling, one approved
guest bootstrap format, existing `justfile` and `infra/ignition` provisioning
assets as evaluated inputs  
**Storage**: Files on disk for scenario definitions, accepted scenario corpus,
repository-evolution fixtures, run workspaces, retained artifact bundles, and
machine-readable run results  
**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
deterministic unit tests for scenario/model logic, integration tests for run
classification and contract output, plus libvirt-backed smoke coverage in
`tests/integration/test_verification_smoke.rs` used as internal validation
support for the authoritative VM-backed path rather than as a substitute for
it  
**Target Platform**: Linux host with libvirt/KVM access and one approved guest
image family  
**Project Type**: Rust CLI application with internal verification modules,
machine-readable output contracts, and a separate development-facing
verification entrypoint  
**Performance Goals**: Accepted CI corpus for v1 remains small and deterministic
enough to produce conclusive results and export artifacts within a bounded CI
window; local runs provide selective one-command execution and diagnosis
without manual cleanup; timing checks stay at coarse upper-bound guardrail
level rather than detailed benchmark analysis  
**Constraints**: Single-VM scenarios only in v1; no implicit internet
dependency during scenario execution; default teardown after artifact
collection; generated scenarios are advisory until accepted; controller version
remains sourced from `Cargo.toml`; implementation should prefer reuse or
extension of the current UAT provisioning path where it fits the harness model,
and migrate manual UAT onto a better replacement path if one is introduced;
feature specifications remain the canonical generation input, with structured
verification guidance treated as optional support rather than a required
intermediate format; scenario authoring should inherit environment and policy
defaults where possible rather than requiring fully explicit common-case
configuration; synthetic or non-VM execution helpers may support internal test
determinism but are not an acceptable replacement for disposable-VM execution  
**Scale/Scope**: v1 targets a curated accepted corpus of tens of scenarios over
one virtualization backend, one guest image family, one VM per scenario run,
and realistic repository-evolution fixtures for public operational command
surfaces

For accepted-corpus and other batch runs, the top-level machine-readable run
result records the batch revision-selection basis, while each scenario outcome
carries its specific revision under test whenever revisions differ across the
batch.

v1 timing guardrails are limited to coarse upper bounds for guest readiness,
scenario-level timeout completion, and selected command-surface operations
whose latency is already part of the public operational contract; detailed
benchmarking remains out of scope.

Release-version-policy notes for this feature:

- the dedicated `core-ops-verify` entrypoint is an externally visible
  development and CI contract even though it does not expand the stable
  operator-facing `core-ops` command surface
- `contracts/scenario-schema.md` and `contracts/run-result-schema.md` are
  public machine-consumed contracts and require conservative evolution review
  before release
- accepted-corpus gating semantics, revision-provenance semantics, and
  documented exit-behavior interpretation are compatibility-sensitive and must
  not change silently
- the canonical controller version reported by verification output remains the
  package version declared in `Cargo.toml`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; scenario schema
  validation, assertion evaluation, taxonomy checks, candidate filtering, and
  result classification remain pure, while libvirt, SSH, filesystem, and
  artifact retention stay in boundary modules.
- Desired/observed state, reconciliation plans, and outcomes are represented as
  data. This feature extends that pattern with declarative scenario definitions,
  run records, assertion results, artifact manifests, and candidate scenario
  metadata.
- Abstractions are minimal and justified. The design adds one cohesive
  verification subsystem rather than a new framework layer.
- Effects, assumptions, and failure modes are explicit in interfaces and
  returns. Run outcomes distinguish assertion failure, infrastructure failure,
  timeout, and harness error.
- Idempotence and convergence strategy are defined. Scenario execution is
  disposable and repeatable against pinned inputs; reruns of accepted scenarios
  should converge on the same outcome classification.
- Open standards and native interfaces are preferred. The harness uses native
  libvirt tooling, guest SSH, filesystem artifacts, and a dedicated
  verification-tool entrypoint instead of opaque proprietary services.
- Observability plan covers scenario definitions, harness logs, VM definitions,
  console logs, CoreOps outputs, assertion results, and failure-specific
  artifacts for offline diagnosis.
- Provenance and status surfaces identify reconciler revision, desired-state
  revision under test, scenario identity, and run outcome in machine-readable
  form.
- Safe defaults are documented. Runs tear down by default after artifact
  capture, debug retention is explicit, and generated scenarios do not gate CI
  until accepted.
- Compatibility impact is assessed. This feature adds new scenario and run
  result contracts plus a dedicated verification-tool surface without changing
  the stable operator-facing `plan`/`apply` contract set.
- Release version policy impact is assessed. New externally visible scenario
  and result schemas require conservative evolution and version review; the
  canonical controller version remains in `Cargo.toml`.
- Release-version review outcome for v1: introducing the dedicated
  `core-ops-verify` entrypoint plus the public scenario and machine-readable
  run-result contracts requires a MINOR version increment unless it ships as
  part of an already-planned MINOR release containing the same feature set.
- Follow-up review rule: additive optional fields may remain compatible, but
  changes to required fields, enum meanings, exit semantics, or documented
  verification behavior require explicit compatibility review and version
  policy evaluation before release.
- Rust changes include the required validation gate plan: `cargo test` and
  `cargo clippy --all-targets -- -D warnings`.
- Test strategy covers invariants, external behavior, repeatability, output
  contracts, and failure classification.
- Modules remain regenerable from spec, contracts, and tests through
  data-oriented design and explicit boundaries.

**Post-Design Re-Check**: PASS. Phase 1 artifacts retain pure model contracts,
explicit effect boundaries, deterministic run classification, conservative
contract evolution, and the required Rust validation gates without needing
constitutional exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/008-e2e-verification-harness/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── scenario-schema.md
│   └── run-result-schema.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── bin/
│   └── core-ops-verify.rs       # Dedicated dev/test verification entrypoint
├── cli/
│   └── verification.rs          # Shared verification command wiring/helpers
├── core/
│   ├── verification_model.rs    # Scenario schema, repository-evolution fixtures, taxonomy, assertion model
│   ├── verification_eval.rs     # Pure assertion evaluation, semantic action planning, command-surface checks, timing guardrails, and run classification logic
│   └── verification_generate.rs # Spec-driven candidate derivation/normalization/filtering
├── io/
│   ├── libvirt.rs               # virsh/qemu-img boundary
│   ├── guest.rs                 # SSH/guest command boundary
│   └── verification_artifacts.rs
└── lib.rs

tests/
├── unit/
│   ├── test_verification_model.rs
│   ├── test_verification_execution.rs
│   ├── test_verification_generation.rs
│   └── test_verification_results.rs
├── integration/
│   ├── test_verification_cli.rs
│   ├── test_verification_contracts.rs
│   ├── test_verification_execution.rs
│   ├── test_verification_generation.rs
│   ├── test_quickstart_verification.rs
│   └── test_verification_smoke.rs
└── fixtures/
    └── verification/
        ├── scenarios/           # Accepted scenarios plus repository-history fixtures
        ├── generated_candidates/
        └── artifacts/
```

**Structure Decision**: Keep the repository as a single Rust project and add a
focused verification subsystem split across `src/core`, `src/io`, and
`src/cli`, with a dedicated `src/bin/core-ops-verify.rs` entrypoint for
development and CI use. This preserves explicit boundary layers while avoiding
accidental expansion of the stable operator-facing `core-ops` CLI.

## Phase 0: Research Summary

Research decisions are documented in
[research.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/research.md).
All previously implicit technical choices have been resolved:

- native libvirt CLI boundaries over a libvirt Rust binding
- declarative YAML-backed scenario definitions with typed Rust parsing
- named environment/policy profiles plus structured semantic actions for
  authorable common-case scenarios
- artifact-first default teardown with explicit debug retention
- accepted-corpus-only CI gating
- feature specifications remain the canonical semantic generation input, with
  optional structured verification guidance used only to improve prompting and
  review quality
- repository-evolution fixtures and public command-surface contracts are
  first-class verification inputs, while detailed performance benchmarking
  remains out of scope
- existing `justfile`, `infra/ignition`, and `VM Host Preparation (CoreOS)`
  workflow treated as reuse candidates and migration inputs
- separate scenario and run-result contracts for conservative public evolution

## Phase 1: Design Artifacts

- [data-model.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/data-model.md)
- [scenario-schema.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/contracts/scenario-schema.md)
- [run-result-schema.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/contracts/run-result-schema.md)
- [quickstart.md](/home/outergod/code/github.com/outergod/core-ops/specs/008-e2e-verification-harness/quickstart.md)

## Complexity Tracking

No constitutional violations or extra complexity exceptions are required for
this design.
