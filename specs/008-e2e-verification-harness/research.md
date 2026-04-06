# Phase 0 Research: E2E Verification Harness with LLM-Assisted Scenario Generation

## Decision 1: Use native libvirt tooling as the virtualization boundary

- **Decision**: Use native libvirt command-line tooling (`virsh`, `qemu-img`,
  and related host utilities) behind explicit boundary adapters rather than a
  dedicated Rust libvirt binding.
- **Rationale**: This aligns with the constitution's preference for native
  interfaces, keeps provisioning effects observable, avoids introducing a large
  new binding dependency, and matches CoreOps' current pattern of explicit
  subprocess boundaries for host integration.
- **Alternatives considered**:
  - Rust libvirt binding: rejected because it adds another abstraction layer
    and makes host-debugging less transparent.
  - External orchestration service: rejected because it adds unnecessary
    infrastructure and weakens offline diagnosability.

## Decision 2: Store scenarios as declarative YAML files parsed into typed Rust data

- **Decision**: Express accepted scenarios and candidate scenarios as
  human-reviewable YAML documents parsed into strongly typed Rust models.
- **Rationale**: YAML is already present in the dependency stack, works well
  for declarative review workflows, and allows schema validation plus
  conservative public evolution through documented contracts.
- **Alternatives considered**:
  - JSON only: rejected because it is less review-friendly for hand-authored
    scenario corpora.
  - Rust-only fixtures: rejected because they would weaken the "spec-derived,
    declarative verification" goal.

## Decision 3: Keep run classification and assertion evaluation pure

- **Decision**: Model scenario validation, coverage taxonomy enforcement,
  assertion evaluation, and final run classification as pure data
  transformations over collected observations.
- **Rationale**: This preserves regenerability, makes outcome semantics easy to
  test, and isolates nondeterminism to provisioning, guest command, and
  artifact collection boundaries.
- **Alternatives considered**:
  - Inline classification in execution orchestration: rejected because it would
    mix side effects and correctness logic.

## Decision 4: Default to teardown after artifact collection; allow explicit debug retention

- **Decision**: All normal runs tear down disposable environments after
  artifact collection, including failed runs. Debug mode may keep the
  environment alive for manual inspection.
- **Rationale**: Default teardown prevents resource leaks and keeps CI and
  local workflows predictable, while artifact retention still enables offline
  diagnosis. Debug mode preserves manual inspection as an explicit opt-in.
- **Alternatives considered**:
  - Always preserve failed environments: rejected because it complicates
    cleanup, increases resource drift, and makes CI less reliable.
  - Never preserve environments: rejected because it blocks interactive
    investigation for complex failures.

## Decision 5: Gate only on accepted scenario corpus entries

- **Decision**: CI and release gates use only accepted scenarios from the
  maintained corpus. Generated scenarios remain advisory until reviewed and
  accepted.
- **Rationale**: This preserves deterministic gating, prevents unstable or
  redundant generated candidates from blocking delivery, and keeps human review
  in the loop for scenario value and safety.
- **Alternatives considered**:
  - Run generated scenarios as blocking gates: rejected because it would make
    gate behavior unstable and hard to audit.
  - Ignore generation in CI entirely: rejected because accepted generated
    scenarios are still part of the long-term corpus strategy.

## Decision 6: Scope v1 to single-VM scenarios on one approved image family

- **Decision**: v1 supports only single-VM scenarios on one approved guest
  image family and one approved virtualization backend.
- **Rationale**: This keeps provisioning, networking, and failure
  classification tractable while still covering convergence, idempotency, drift
  correction, reboot resilience, explain/apply consistency, and upgrade
  transition scenarios.
- **Alternatives considered**:
  - Multi-VM topologies in v1: rejected because they would expand orchestration
    and failure-mode complexity too early.
  - Multiple image families in v1: rejected because they reduce repeatability
    and broaden the support matrix before the harness contract is stable.

## Decision 7: Separate public contracts for scenario schema and run-result schema

- **Decision**: Define and version two explicit public contracts: one for the
  declarative scenario schema and one for machine-readable verification run
  results.
- **Rationale**: The feature introduces both a user-authored input surface and
  an automation-facing output surface; documenting them separately supports
  conservative evolution and clearer compatibility review.
- **Alternatives considered**:
  - One combined contract document: rejected because it would blur authoring
    rules with emitted result semantics.

## Decision 8: Use deterministic internal tests alongside authoritative VM-backed verification

- **Decision**: Keep unit and integration tests deterministic in the normal
  suite, and use libvirt-backed smoke coverage only as internal validation
  support for the authoritative disposable-VM execution path.
- **Rationale**: The constitution requires stable validation gates. Core logic
  and contracts can be covered by `cargo test` and `cargo clippy`, while the
  feature’s intended outcome remains VM-backed end-to-end verification rather
  than any synthetic backend.
- **Alternatives considered**:
  - Require libvirt-backed tests in every default test run: rejected because it
    would make validation environment-sensitive and harder to reproduce.

## Decision 9: Treat existing UAT provisioning assets as research inputs and reuse candidates

- **Decision**: Treat the existing `justfile`, `infra/ignition` templates, and
  the documented `VM Host Preparation (CoreOS)` workflow in
  `docs/development.md` as first-class research inputs and candidate
  implementation substrate for the verification harness.
- **Rationale**: These assets already represent the project's current manual
  UAT provisioning path. Evaluating them first reduces duplicated provisioning
  logic, keeps manual and automated verification closer together, and leaves
  room to either reuse/extend them directly or replace them with a better
  harness path and migrate UAT onto that path.
- **Alternatives considered**:
  - Ignore existing UAT tooling and build a separate harness path: rejected
    because it risks reinventing provisioning workflows and creating two
    divergent ways to prepare test systems.
  - Mandate direct reuse without evaluation: rejected because the harness may
    need a cleaner abstraction than the current manual workflow exposes.
