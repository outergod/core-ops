# Implementation Plan: Serial Console Readiness

**Branch**: `009-serial-console-readiness` | **Date**: 2026-04-07 | **Spec**: [spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/spec.md)
**Input**: Feature specification from `/specs/009-serial-console-readiness/spec.md`

## Summary

Replace ARP-first guest address discovery in the VM-backed verification harness
with a guest self-reported readiness record emitted on the serial console,
validated against the current run identity, and used as the authoritative guest
IPv4 source for later guest-boundary work. The implementation should keep
readiness record parsing, validation, and failure classification pure while
isolating ignition rendering, console-log collection, and libvirt/SSH
interaction inside the existing harness boundaries. ARP-based discovery remains
available only as a temporary rollout fallback and must never outrank a valid
current-run readiness record.

## Technical Context

**Language/Version**: Rust 2021  
**Primary Dependencies**: Existing CoreOps Rust stack (`clap`, `miette`,
`thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `tempfile`), native
libvirt tooling (`virsh`, `qemu-img`), existing Butane/Ignition rendering path,
SSH client tooling, existing serial-console artifact capture in the
verification harness  
**Storage**: Files on disk for rendered ignition inputs, serial console logs,
verification workspaces, retained artifact bundles, and machine-readable run
results  
**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`, unit
tests for readiness-record parsing/validation, integration tests for run
classification and fallback precedence, and existing VM-backed smoke/support
coverage where appropriate  
**Target Platform**: Linux host with libvirt/KVM access and the approved
CoreOS-derived guest image path used by the existing verification harness  
**Project Type**: Rust CLI application with verification-harness modules and
machine-readable run contracts  
**Performance Goals**: A healthy VM-backed run should become reachable using a
valid serial-console readiness record within the configured readiness window,
without requiring opportunistic neighbor-cache observation; invalid console
records must fail fast or be ignored without extending the normal readiness
path unpredictably  
**Constraints**: VM-backed verification only; readiness record provides a
usable IPv4 address only; serial-console readiness is primary; ARP remains
temporary migration fallback only; run identity matching is mandatory; failure
outcomes must remain distinguishable from behavioral CoreOps failures;
controller version remains sourced from `Cargo.toml`  
**Scale/Scope**: One readiness record per guest run, one guest per scenario in
v1, tens of accepted scenarios, one approved virtualization backend, and a
single focused enhancement to guest readiness/address acquisition rather than a
general guest agent system

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit. Readiness
  record parsing, validation, fallback precedence, and outcome classification
  remain pure; ignition injection, console-log retrieval, and guest/libvirt
  interaction remain in boundary modules.
- Desired/observed state, reconciliation plans, and outcomes remain data.
  This feature adds a run-scoped readiness record and readiness acquisition
  state without changing declarative workload semantics.
- Abstractions are minimal and justified. The design extends existing
  verification/libvirt boundaries rather than adding a broader guest-agent
  framework.
- Effects, assumptions, and failure modes are explicit in interfaces and
  returns. Missing, malformed, and stale readiness are modeled as explicit
  readiness failures.
- Idempotence and convergence strategy are defined. Re-reading the same valid
  readiness record does not change the selected guest identity; invalid records
  do not advance run state.
- Open standards and native interfaces are preferred. The design uses serial
  console output, systemd, Ignition, and libvirt artifacts rather than a
  proprietary side channel.
- Observability plan covers readiness acceptance/rejection, timeout behavior,
  serial-console evidence, and run-result failure summaries.
- Provenance and status surfaces identify current run identity, desired-state
  revision under test, and readiness-related outcomes in machine-readable form.
- Safe defaults are documented. A valid current-run readiness record is trusted
  first; ARP fallback is temporary and subordinate.
- Compatibility impact is assessed. This changes VM-backed harness runtime
  behavior and readiness failure semantics without expanding the stable
  operator-facing `core-ops` CLI.
- Release version policy impact is assessed. This is an externally observable
  verification-behavior change and requires conservative compatibility review;
  canonical controller version remains sourced from `Cargo.toml`.
- Rust changes include the required validation gate plan: `cargo test` and
  `cargo clippy --all-targets -- -D warnings`.
- Test strategy covers invariants, external behavior, timeout/failure
  semantics, and migration fallback precedence.
- Modules remain regenerable from spec, contracts, and tests through
  data-oriented readiness contracts and explicit boundary logic.

**Post-Design Re-Check**: PASS. Phase 1 artifacts preserve explicit side-effect
boundaries, stable readiness contracts, conservative behavior evolution, and
the required Rust validation gates without requiring constitutional exceptions.

## Project Structure

### Documentation (this feature)

```text
specs/009-serial-console-readiness/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── readiness-record-contract.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── report.rs                # Human and machine-readable verification run reporting
│   └── verification.rs          # Scenario execution, artifact capture, readiness orchestration
├── core/
│   ├── boundaries.rs            # Verification boundary traits
│   ├── verification_eval.rs     # Pure run classification and timeout semantics
│   └── verification_model.rs    # Verification data contracts and guest handles
├── io/
│   ├── guest.rs                 # Guest-boundary SSH and readiness probing
│   ├── libvirt.rs               # VM provisioning, ignition rendering, console-log access, address discovery
│   └── verification_artifacts.rs
└── lib.rs

tests/
├── integration/
│   ├── test_verification_execution.rs
│   ├── test_verification_cli.rs
│   └── test_verification_contracts.rs
└── unit/
    ├── test_verification_model.rs
    ├── test_verification_execution.rs
    └── test_verification_results.rs
```

**Structure Decision**: Keep the repository as a single Rust project and
implement the change inside the existing verification subsystem. The focused
touch points are `src/io/libvirt.rs` for readiness acquisition and ignition
payload shaping, `src/cli/verification.rs` for orchestration and artifact
handling, and existing pure verification modules for readiness-related outcome
classification.

## Phase 0: Research Summary

Research decisions are documented in
[research.md](/home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/research.md).
All technical unknowns in this feature have been resolved:

- serial-console readiness is the primary VM-backed readiness/address contract
- the readiness payload is a single structured line carrying run id, token,
  IPv4 address, and optional diagnostic fields
- run-scoped token matching is required to reject stale console data
- existing ignition templating is the smallest insertion point for the guest
  script/service payload
- ARP remains a temporary fallback only during migration and never outranks a
  valid current-run readiness record

## Phase 1: Design Artifacts

- [data-model.md](/home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/data-model.md)
- [readiness-record-contract.md](/home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/contracts/readiness-record-contract.md)
- [quickstart.md](/home/outergod/code/github.com/outergod/core-ops/specs/009-serial-console-readiness/quickstart.md)

## Complexity Tracking

No constitutional violations or extra complexity exceptions are required for
this design.

## Release Version Policy Review

- The implementation changes `core-ops-verify` runtime behavior, failure
  classification, artifacts, and machine-readable run payloads for VM-backed
  readiness acquisition.
- The stable operator-facing `core-ops` command surface is unchanged.
- Compatibility-sensitive elements are the readiness marker, required record
  fields, readiness-evidence artifact shape, and timeout versus
  infrastructure-style readiness failure semantics.
