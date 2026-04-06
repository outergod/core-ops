# Feature Specification: E2E Verification Harness with LLM-Assisted Scenario Generation

**Feature Branch**: `008-e2e-verification-harness`  
**Created**: 2026-04-01  
**Status**: Draft  
**Input**: User description: "Use `.agent/spec.md` as input."

## Clarifications

### Session 2026-04-01

- Q: Which scenarios are allowed to gate CI and release decisions? → A: Only accepted scenarios from the maintained corpus may gate CI and release decisions; generated candidates remain advisory until reviewed and accepted.
- Q: What VM topology scope is supported in v1? → A: v1 supports only single-VM scenarios; multi-VM scenarios are out of scope.
- Q: Should failed runs preserve the disposable environment for manual inspection by default? → A: Default runs tear down the environment after artifact collection; debug mode may retain it for manual inspection.

### Session 2026-04-02

- Q: What is the semantic role of feature specifications versus verification-oriented fields in scenario generation? → A: Feature specifications are the canonical semantic source. Verification-oriented fields provide structured guidance but MUST NOT be required to fully reconstruct behavioral meaning.
- Q: Should the system help identify missing coverage across scenario classes for a feature or corpus? → A: The system SHOULD make it possible to identify missing coverage across scenario classes for a given feature or corpus.
- Q: What counts as part of the public operational contract for command-surface verification? → A: Public operational contract includes outputs and behaviors relied upon by users, automation, or external systems, including machine-readable formats, exit semantics, and documented CLI behavior.

### Session 2026-04-06

- Q: What is the authoritative execution mode for this feature? → A: VM-backed disposable-machine execution is the authoritative verification mode. Any synthetic or non-VM backend is internal test support only and does not satisfy the feature on its own.

## Verification System Scope

CoreOps must gain an executable verification system with these subsystems:

- Scenario Model
- Repository Evolution Model
- Runtime Harness
- Command and Output Contract Verification
- Scenario Generation Pipeline
- Regression Corpus
- Developer and CI Integration
- Operational Timing Guardrails

The scenario model SHOULD separate three layers of concern:

- behavioral intent
- environment profile selection
- harness policy overrides

Common-case scenarios SHOULD stay short and authorable by inheriting standard
profiles and defaults rather than restating routine harness configuration.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run Disposable End-to-End Verification (Priority: P1)

As a CoreOps developer or release engineer, I want to run end-to-end
verification scenarios against disposable machines so that I can prove CoreOps
behavior under real runtime conditions instead of relying only on unit or
integration tests.

**Why this priority**: This is the minimum useful slice. Without executable
runtime verification, the feature does not deliver its primary value.

**Independent Test**: Can be fully tested by executing a declared scenario
against a disposable environment, collecting artifacts, and determining a pass
or fail outcome from deterministic assertions.

**Acceptance Scenarios**:

1. **Given** a declared scenario with environment setup, steps, assertions, and
   timeouts, **When** an operator runs the harness, **Then** the system executes
   the scenario in an isolated disposable environment and reports a conclusive
   pass or fail result.
2. **Given** a scenario that fails an assertion, **When** the harness finishes,
   **Then** the result identifies the failed assertion and preserves the
   artifacts needed for offline diagnosis.

---

### User Story 2 - Derive Candidate Scenarios from Feature Specifications (Priority: P2)

As a spec author or reviewer, I want candidate verification scenarios derived
from feature specifications so that verification coverage starts from declared
behavior instead of ad hoc manual test writing.

**Why this priority**: This expands coverage and keeps verification aligned with
specifications, but the harness itself must exist first.

**Independent Test**: Can be fully tested by submitting a feature
specification, receiving candidate scenarios with declared coverage categories
and behavioral claims, and confirming only accepted scenarios become runnable
inputs.

**Acceptance Scenarios**:

1. **Given** a feature specification with observable behaviors and invariants,
   **When** candidate scenarios are generated, **Then** each candidate includes
   a declared behavioral claim, coverage classification, and deterministic
   assertions.
2. **Given** a malformed, redundant, or unstable generated candidate, **When**
   it enters validation, **Then** the system rejects it before it can be
   treated as part of the accepted scenario corpus.

---

### User Story 3 - Gate Revisions and Diagnose Regressions (Priority: P3)

As a release engineer or CI operator, I want verification runs to gate revisions
and preserve rich diagnostics so that regressions can be detected and
investigated without rerunning the same environment immediately.

**Why this priority**: This turns the harness into an operational quality gate,
but it depends on both executable scenarios and curated scenario coverage.

**Independent Test**: Can be fully tested by running the harness in a
non-interactive mode against a revision under review and confirming it emits
deterministic exit status plus an artifact bundle sufficient for offline
analysis.

**Acceptance Scenarios**:

1. **Given** a non-interactive verification run, **When** all scenarios pass,
   **Then** the run exits cleanly and publishes machine-readable results and
   retained artifacts.
2. **Given** a run that encounters behavioral failure, infrastructure failure,
   or timeout, **When** the run completes, **Then** the outcome classification
   distinguishes those cases and preserves the corresponding diagnostics.

### Edge Cases

- What happens when the disposable environment becomes unreachable before all
  assertions complete?
- How does the system handle generated candidate scenarios that duplicate
  existing coverage but use different wording?
- What happens when a scenario depends on unsupported infrastructure or an
  unsupported guest image family?
- How does the harness behave when a reboot or state mutation step succeeds but
  the subsequent readiness condition never becomes true?
- What happens when artifact collection partially fails after the primary
  scenario outcome is already known?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST execute end-to-end verification scenarios against
  disposable machine environments rather than only simulated or in-process
  state.
- **FR-001a**: VM-backed disposable-machine execution MUST be the
  authoritative verification mode for accepted scenarios; any synthetic,
  simulated, or non-VM backend MAY exist only as internal support for
  deterministic automated validation and MUST NOT be treated as satisfying the
  end-to-end verification objective by itself.
- **FR-002**: The system MUST accept a declarative scenario definition that
  includes environment setup, fixtures, ordered steps, assertions, timeouts,
  and artifact collection policy.
- **FR-002b**: The scenario schema MUST separate behavioral intent from
  environment selection and harness-policy configuration so that common-case
  scenarios do not need to restate routine infrastructure or retention
  defaults inline.
- **FR-002c**: The scenario schema MUST support named environment profiles and
  default harness-policy profiles, with inline overrides only when a scenario
  intentionally deviates from the common path.
- **FR-002a**: The system MUST support scenario execution against repository
  evolution sequences, not only a single revision under test, so that
  realistic revision history with valid and invalid target states can be
  modeled explicitly.
- **FR-003**: The system MUST support, at minimum, steps for booting a machine,
  waiting for readiness, running a CoreOps command, executing a command inside
  the guest, mutating runtime state, and rebooting the guest.
- **FR-003a**: Scenario steps SHOULD express structured semantic actions for
  common CoreOps operations rather than requiring raw CLI spellings inline;
  raw command strings MAY still be supported for explicit guest commands or
  uncommon escape-hatch behavior.
- **FR-004**: The system MUST support, at minimum, assertions for file
  existence or content, managed service state, command exit behavior and
  output, CoreOps command output, absence of pending changes, and expected
  failure classification.
- **FR-005**: The system MUST evaluate scenario success and failure using
  deterministic assertions; generated content or harness heuristics MUST NOT be
  treated as authoritative evidence of correctness.
- **FR-006**: The system MUST assign each scenario to one or more supported
  verification classes, including convergence, idempotency, drift correction,
  dependency ordering, reboot resilience, upgrade transition, failure
  diagnosis, partial apply or recovery, and explain or apply consistency.
- **FR-007**: The system MUST execute each verification run in an isolated
  workspace with a unique run identifier and deterministic resource naming.
- **FR-008**: The system MUST classify failed runs as assertion failure,
  infrastructure failure, timeout, or harness error.
- **FR-009**: The system MUST always preserve the scenario definition, harness
  log, machine definitions, console output, CoreOps command outputs, and
  assertion results for each run.
- **FR-009a**: Artifact retention and timeout configuration SHOULD inherit from
  harness or profile defaults unless a scenario explicitly overrides them.
- **FR-010**: The system MUST preserve additional offline diagnostic artifacts
  for failed runs, including relevant service state, relevant files, explain
  output, and revision identifiers.
- **FR-011**: The system MUST support candidate scenario generation from feature
  specifications, behavioral contracts, existing accepted scenarios, and system
  semantics.
- **FR-011a**: Candidate scenario derivation MUST be driven by the feature
  specification itself as the canonical semantic input; optional structured
  verification guidance inside the spec MAY assist generation but MUST NOT be
  required to fully reconstruct behavioral meaning.
- **FR-012**: The system MUST require each generated candidate scenario to
  declare its coverage classification, rationale, and behavioral claim under
  test.
- **FR-013**: The system MUST reject generated candidate scenarios that lack
  assertions, duplicate existing accepted coverage, depend on unstable signals,
  require unsupported infrastructure, or lack a clear behavioral purpose.
- **FR-014**: Generated scenarios MUST remain advisory until explicitly
  accepted into the maintained scenario corpus.
- **FR-015**: The system MUST support both interactive local execution and
  non-interactive CI execution for CoreOps development, release, and CI
  workflows.
- **FR-015c**: Interactive local execution and non-interactive CI execution
  MUST both remain capable of using the authoritative disposable-VM execution
  path; development convenience backends MUST NOT replace VM-backed execution
  as the intended verification outcome.
- **FR-015b**: The system MUST support selective, incremental, and reproducible
  scenario execution suitable for feature development, debugging, and
  regression triage, not only batch CI execution.
- **FR-015a**: The initial release MUST expose verification workflows through a
  dedicated development or testing entrypoint separate from the stable
  operator-facing `core-ops` command surface.
- **FR-016**: Non-interactive execution MUST emit machine-readable results and
  deterministic exit status suitable for automated gating.
- **FR-016a**: Automated gating decisions MUST be based only on accepted
  scenarios from the maintained scenario corpus; generated candidate scenarios
  MUST NOT block CI or release workflows until they have been reviewed and
  accepted.
- **FR-016b**: Scenarios MAY assert command-surface behavior for CoreOps
  commands whose outputs or side effects are part of the public operational
  contract, meaning outputs and behaviors relied upon by users, automation, or
  external systems, including machine-readable formats, exit semantics,
  documented CLI behavior, and human-readable, interactive, agent, or
  non-interactive interfaces where applicable.
- **FR-016c**: The initial release MUST prioritize command-surface verification
  for human-readable CLI summaries, machine-readable JSON output,
  non-interactive exit semantics, and agent-facing output where it is already
  part of the public operational contract; exhaustive per-command or per-flag
  coverage is out of scope for v1.
- **FR-017**: The system MUST support a debug-oriented execution mode that
  retains more artifacts and diagnostic detail than the default execution mode.
- **FR-017a**: Default execution MUST tear down disposable environments after
  artifact collection completes, including failed runs; a debug-oriented mode
  MAY retain the environment for manual inspection.
- **FR-017b**: Debug-oriented execution MUST support an explicit
  pause-before-teardown workflow for interactive investigation when an
  operator wants temporary live inspection without leaving the disposable
  environment retained indefinitely.
- **FR-018**: The system MUST avoid arbitrary timing dependencies by relying on
  explicit readiness conditions, explicit timeouts, pinned inputs, and minimal
  external dependencies.
- **FR-019**: All CoreOps feature specifications targeted by this verification
  workflow MUST describe observable behaviors, invariants, idempotency
  expectations, failure modes, upgrade considerations, and required scenario
  classes.
- **FR-019a**: Verification-oriented fields in feature specifications MUST
  remain a mandatory, human-authored section for features that participate in
  this workflow, and MUST NOT become a rigid intermediate representation
  required to reconstruct the full behavioral meaning of the feature
  specification.
- **FR-020**: The system MUST make regression detection across revisions
  possible by associating verification runs with the revision under test and
  surfacing behavioral differences through run outcomes and artifacts.
- **FR-020b**: When a verification run executes an accepted corpus or other
  multi-scenario batch, the machine-readable run result MUST identify the
  aggregate revision-selection basis for the batch and preserve per-scenario
  revision-under-test provenance where scenarios do not all target the same
  desired-state revision.
- **FR-020a**: The system MUST support promotion of real bug reproductions into
  permanent accepted regression scenarios so that fixes can be validated and
  future regressions can be detected automatically.
- **FR-021**: The initial release MUST constrain execution to a single approved
  disposable machine environment family and a single approved guest image
  family.
- **FR-021a**: The initial release MUST support only single-VM verification
  scenarios; scenarios requiring multiple coordinated guest machines MUST be
  out of scope for v1.
- **FR-021b**: The system MAY enforce coarse operational timing and
  responsiveness guardrails, including upper-bound latency and timeout
  expectations where operationally meaningful, but detailed performance
  benchmarking MUST remain out of scope for the core verification contract.
- **FR-022**: The system SHOULD make it possible to identify missing coverage
  across scenario classes for a given feature specification or accepted
  scenario corpus.

### Key Entities *(include if feature involves data)*

- **Scenario Definition**: The declarative description of an end-to-end
  verification case, including environment setup, fixtures, steps, assertions,
  timeouts, taxonomy, and artifact policy.
- **Environment Profile**: A named reusable execution profile that selects the
  standard guest, image, network, bootstrap, and connection defaults for a
  scenario.
- **Harness Policy Override**: Optional scenario-local overrides to default
  timeout, artifact-retention, or debug-behavior policies.
- **Repository Evolution Model**: The declarative description of the Git
  history, revision sequence, and valid or invalid target-state transitions a
  scenario exercises.
- **Verification Run**: A single isolated execution of one or more scenarios,
  identified by a unique run ID and associated with a revision under test,
  result status, and retained artifacts.
- **Assertion Result**: The recorded outcome of an individual observable claim,
  including whether it passed, failed, timed out, or could not be evaluated.
- **Candidate Scenario**: A generated but not yet accepted scenario proposal
  that includes a behavioral claim, rationale, and coverage classification.
- **Accepted Scenario Corpus**: The reviewed set of scenarios approved for
  repeated execution and regression detection.
- **Regression Scenario**: An accepted scenario derived from a real bug
  reproduction and retained permanently to protect against recurrence.
- **Artifact Bundle**: The retained diagnostic output for a verification run,
  including always-collected materials and failure-specific evidence.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Scenario validation, taxonomy
  classification, candidate rejection, and assertion evaluation rules can be
  modeled as pure transformations, while machine provisioning, guest commands,
  and artifact collection remain explicit boundary effects.
- **Declarative state model**: Scenarios, assertions, run classifications,
  candidate scenarios, and artifact policies are expressed as explicit data
  rather than hidden procedural conventions.
- **Idempotence & convergence**: Verification runs are disposable and repeatable
  against pinned inputs, allowing stable reruns of the same scenario corpus for
  regression detection.
- **Explicit effects/failures**: The feature requires explicit distinction among
  assertion failure, infrastructure failure, timeout, and harness error.
- **Observability**: The harness is centered on diagnosable artifacts, explicit
  run outcomes, and offline debugging evidence.
- **Provenance & traceability**: Verification runs are tied to the revision
  under test, scenario definitions, and retained artifacts so observed behavior
  can be compared across revisions.
- **Safe defaults**: Disposable execution, isolated environments, explicit
  artifact policies, and reviewed candidate acceptance reduce the risk of
  accidental or misleading verification behavior.
- **Compatibility**: The feature introduces new verification surfaces without
  replacing existing unit or integration testing expectations, and without
  expanding the stable operator-facing `core-ops` command surface in v1.
- **Release version policy**: Any externally visible scenario schema, run
  result schema, verification-tool entrypoint contract, or gating behavior
  must follow the project’s versioning policy and preserve `Cargo.toml` as the
  canonical controller version source.
- **Test contract**: The delivered implementation must validate invariants,
  scenario parsing, execution classification, artifact retention, and for Rust
  changes the required `cargo test` and
  `cargo clippy --all-targets -- -D warnings` gates or a documented exemption.
- **Regenerability**: Scenarios derive from explicit specifications and
  contracts, allowing the harness behavior to be regenerated from declarative
  inputs and tests.

## Assumptions

- The first release will support one approved disposable virtualization backend
- The first release will use one approved guest image family with pinned image
  versions
- The first release will execute only single-VM scenarios
- The first release will support authored repository-history fixtures as the
  baseline repository-evolution mechanism; generic generated Git histories may
  be added later as a secondary path
- The first release should optimize the scenario schema for authorability in
  the common case by relying on named profiles and omitted-default fields where
  possible
- Operators and CI will both use the same scenario model, with execution mode
  changing artifact retention and interaction style rather than changing
  scenario meaning
- The first release will keep verification execution on a dedicated internal or
  development-facing entrypoint instead of the stable operator-facing
  `core-ops` binary surface
- Default runs tear down disposable environments after artifact collection,
  while debug mode may retain them for manual inspection
- Candidate scenarios are reviewed by humans before joining the accepted corpus
- Feature specifications remain the canonical generation input; any structured
  verification guidance is optional support for generation quality and review,
  not a required substitute for semantic reading of the spec
- The initial release should prioritize public operational command surfaces and
  coarse timing guardrails over exhaustive command or option coverage
- Verification focuses on behavioral correctness rather than load or
  performance benchmarking

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Operators can execute an accepted end-to-end verification
  scenario and receive a conclusive pass or fail result with retained artifacts
  in a single run without manual environment cleanup.
- **SC-002**: 100% of failed verification runs preserve enough diagnostics for a
  reviewer to determine whether the outcome was an assertion failure,
  infrastructure failure, timeout, or harness error without rerunning the same
  scenario immediately.
- **SC-003**: Accepted scenarios produce the same outcome classification in at
  least 95% of repeated runs against the same pinned inputs and revision under
  test.
- **SC-004**: 100% of generated candidate scenarios that enter review declare a
  behavioral claim, rationale, and scenario taxonomy classification.
- **SC-005**: CI-mode execution can gate a revision using machine-readable run
  output and deterministic exit status without requiring interactive
  interpretation.
- **SC-006**: Developers can run a focused subset of accepted or candidate
  scenarios reproducibly against a chosen repository history without needing to
  execute the entire corpus.
- **SC-007**: Real bug reproductions can be promoted into the accepted corpus
  and rerun successfully as regression scenarios after the corresponding fix is
  implemented.
- **SC-008**: Reviewers can determine which required scenario classes for a
  feature remain uncovered by the current accepted corpus without manually
  inspecting every scenario definition.
