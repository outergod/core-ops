# Feature Specification: Serial Console Readiness

**Feature Branch**: `009-serial-console-readiness`  
**Created**: 2026-04-07  
**Status**: Draft  
**Input**: User description: "Use `docs/verification-serial-console-readiness-proposal.md` as input."

## Clarifications

### Session 2026-04-07

- Q: Should ARP remain available once serial-console readiness is introduced? → A: Keep ARP as a temporary fallback during rollout, but prefer serial-console readiness whenever a valid record exists.
- Q: What address family must the readiness record provide for this feature? → A: The readiness record only needs to provide a usable IPv4 address.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reach Guests Reliably (Priority: P1)

As an operator running VM-backed verification, I need the harness to learn the
guest's reachable address from the guest itself so healthy runs do not stall or
fail because of unreliable network-neighbor observation.

**Why this priority**: This removes a known source of false failures and stalled
verification runs in the authoritative VM-backed path.

**Independent Test**: Can be fully tested by running a VM-backed verification
scenario in an environment where neighbor-cache timing is unreliable and
confirming the harness still reaches the guest through a guest-published
readiness record.

**Acceptance Scenarios**:

1. **Given** a VM-backed verification run starts a healthy guest, **When** the
   guest publishes a readiness record for that run, **Then** the harness uses
   that record to continue guest-boundary work without requiring ARP-derived
   address discovery.
2. **Given** the guest becomes network-ready, **When** the harness receives a
   valid readiness record, **Then** the run proceeds using the guest IPv4
   address contained in that record as the authoritative address for later
   steps.

---

### User Story 2 - Reject Stale Or Wrong Readiness Records (Priority: P2)

As an operator, I need the harness to reject readiness records from earlier or
unrelated runs so a new verification run cannot accidentally connect to the
wrong guest.

**Why this priority**: Run identity safety matters almost as much as reachability
because accepting stale readiness would make results untrustworthy.

**Independent Test**: Can be fully tested by presenting the harness with a
readiness record whose run identity does not match the current run and
confirming the run does not proceed from that record.

**Acceptance Scenarios**:

1. **Given** a console-visible readiness record from a previous run exists,
   **When** a new run waits for readiness, **Then** the harness ignores the
   stale record and continues waiting for one that matches the current run.
2. **Given** a readiness record contains an invalid or mismatched run identity,
   **When** the harness evaluates it, **Then** the harness rejects it and does
   not treat the guest as ready.

---

### User Story 3 - Fail Explicitly When Readiness Never Arrives (Priority: P3)

As an operator, I need missing or malformed guest readiness to fail with a
clear run outcome so I can distinguish guest-startup problems from behavioral
CoreOps failures.

**Why this priority**: Clear failure reporting protects CI gating and reduces
time spent diagnosing harness-level failures.

**Independent Test**: Can be fully tested by running a VM-backed verification
scenario in which no valid readiness record appears and confirming the run ends
with an explicit timeout or infrastructure-style readiness failure.

**Acceptance Scenarios**:

1. **Given** a VM-backed verification run starts a guest that never publishes a
   valid readiness record, **When** the configured readiness window expires,
   **Then** the run ends with an explicit readiness-related failure outcome.
2. **Given** a guest publishes malformed readiness output, **When** the harness
   cannot extract a valid readiness record, **Then** the run does not proceed as
   if the guest were ready and surfaces a diagnosable failure.

### Edge Cases

- If multiple valid readiness records appear for the same run, the first
  accepted record remains authoritative and later records are retained only as
  diagnostics.
- How does the harness behave when a readiness record appears but omits a usable
  guest IPv4 address?
- What happens when a previous run's console output is still present in the log
  location inspected by the current run?
- How does the harness behave when the guest reaches network readiness after the
  configured readiness timeout has already expired?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The verification harness MUST treat a guest-published
  console-visible readiness record as the primary readiness and address-discovery
  signal for VM-backed verification runs.
- **FR-002**: The readiness record MUST include a run-specific identity that can
  be matched to the current verification run.
- **FR-003**: The harness MUST reject readiness records whose run-specific
  identity does not match the current verification run.
- **FR-004**: The harness MUST reject malformed readiness records and MUST NOT
  treat a guest as ready unless the record can be parsed and validated.
- **FR-005**: The harness MUST use the guest IPv4 address contained in the
  accepted readiness record as the authoritative guest address for later
  guest-boundary steps in that run.
- **FR-006**: The harness MUST wait for a valid readiness record only within a
  bounded readiness window and MUST surface an explicit failure when that window
  expires.
- **FR-007**: The readiness-related failure outcome MUST remain distinguishable
  from behavioral CoreOps verification failures in run reporting.
- **FR-008**: During migration, the harness MAY retain a fallback address
  discovery path, but it MUST prefer the guest-published readiness record when a
  valid record is available, and ARP-based discovery MUST remain only a
  temporary rollout fallback rather than the primary contract.
- **FR-009**: The harness MUST capture enough readiness evidence in run
  artifacts or reports for operators to determine whether a run succeeded,
  timed out, or rejected stale or malformed readiness data.
- **FR-010**: The harness MUST accept the first valid readiness record for the
  current run and MUST ignore later readiness records for that same run while
  retaining them as diagnostic evidence.

### Key Entities *(include if feature involves data)*

- **Readiness Record**: A single guest-published readiness message that
  identifies a specific run and reports the guest IPv4 address to use for later
  verification steps.
- **Run Identity**: The run-scoped values that bind a readiness record to one
  verification run and prevent stale-record reuse.
- **Readiness Window**: The bounded period during which the harness waits for a
  valid readiness record before reporting failure.

### Assumptions

- VM-backed verification remains the authoritative execution path for this
  feature area.
- Later guest-boundary checks still occur after readiness; this feature only
  changes how initial guest readiness and address discovery are established.
- Existing run artifacts already capture enough console or run diagnostics to
  extend them with readiness evidence without changing the operator workflow.

## Verification Guidance *(mandatory for features that participate in the verification workflow)*

### Observable Behaviors

- A healthy guest emits one valid readiness record that the harness accepts for
  the current run.
- The harness uses the accepted readiness record to continue the run without
  depending primarily on ARP-derived address discovery.
- The accepted readiness record supplies the authoritative IPv4 address for the
  run.
- Stale, mismatched, or malformed readiness records do not unblock the run.
- Missing readiness produces an explicit timeout or infrastructure-style
  readiness failure.

### Invariants

- A readiness record for one run MUST NOT satisfy readiness for another run.
- A guest MUST NOT be considered ready unless its readiness record is both valid
  and bound to the current run.
- Behavioral CoreOps failures after readiness MUST remain distinguishable from
  readiness acquisition failures.

### Idempotency Expectations

- Re-reading the same valid readiness record for the current run MUST NOT
  change the chosen guest identity or chosen IPv4 address after readiness is
  established.
- Reprocessing stale or malformed readiness records MUST NOT advance run state.

### Failure Modes

- No valid readiness record appears before the readiness window expires.
- A readiness record appears but cannot be parsed or validated.
- A readiness record belongs to another run and is rejected.
- A readiness record is accepted but later guest-boundary work still fails for
  unrelated infrastructure reasons.

### Upgrade Considerations

- Existing VM-backed scenarios that previously depended on ARP-first discovery
  MUST continue to run under the new readiness contract.
- During rollout, ARP-based fallback MAY remain temporarily, but it MUST NOT
  take precedence over a valid current-run readiness record and MUST be treated
  as migration-only behavior.
- Run-result reporting and artifacts MUST remain stable enough for existing
  verification tooling to distinguish readiness failures from behavioral
  assertion failures.

### Required Scenario Classes

- guest_readiness_success
- stale_readiness_rejection
- malformed_readiness_rejection
- missing_readiness_timeout

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Readiness record validation, matching,
  and outcome classification remain deterministic decision logic; guest startup,
  console observation, and later guest access remain explicit side-effecting
  boundaries.
- **Declarative state model**: The feature adds a run-scoped readiness record
  and readiness outcome semantics without changing the desired-state model for
  managed workloads.
- **Idempotence & convergence**: A valid readiness record deterministically
  selects the guest identity for the run; repeated reads of the same record do
  not change run meaning.
- **Explicit effects/failures**: Missing, malformed, or stale readiness is
  surfaced as an explicit readiness acquisition failure rather than an implicit
  neighbor-cache problem.
- **Observability**: Readiness acceptance, rejection, timeout, and evidence are
  visible in operator-facing diagnostics and machine-readable run outputs.
- **Provenance & traceability**: Each accepted readiness record is bound to the
  current run identity so later guest actions can be traced back to the run that
  established readiness.
- **Safe defaults**: The harness prefers a validated current-run readiness
  record over opportunistic address inference and does not trust ambiguous
  console data.
- **Compatibility**: The change is scoped to VM-backed verification readiness
  behavior; migration fallback may remain temporarily, but the new record is the
  preferred contract.
- **Release version policy**: This changes a public verification-harness runtime
  behavior and failure contract, so version impact must be reviewed under the
  existing release policy for externally visible CLI and verification behavior.
- **Test contract**: Coverage must include successful readiness acquisition,
  stale-record rejection, malformed-record rejection, and missing-readiness
  timeout, plus the standard `cargo test` and
  `cargo clippy --all-targets -- -D warnings` gates unless explicitly exempted.
- **Regenerability**: The spec defines stable behavioral contracts and scenario
  classes so future verification scenarios and tests can be regenerated safely.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a VM-backed verification run with a healthy guest, the harness
  can determine the guest IPv4 address from a valid readiness record without
  relying primarily on network-neighbor observation.
- **SC-002**: In validation runs containing stale or mismatched readiness
  records, 100% of those records are rejected and none incorrectly unblock the
  current run.
- **SC-003**: In validation runs where no valid readiness record appears, 100%
  of runs end with an explicit readiness-related failure outcome within the
  configured readiness window.
- **SC-004**: Operators can distinguish readiness acquisition failures from
  behavioral CoreOps verification failures using run outputs and artifacts
  without needing to inspect host neighbor-cache state.
