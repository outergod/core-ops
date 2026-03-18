# Feature Specification: GitOps Quadlet Controller

**Feature Branch**: `[001-gitops-quadlet-controller]`  
**Created**: 2026-03-18  
**Status**: Draft  
**Input**: User description: "Build an open-source GitOps-style controller for Fedora CoreOS that manages host-level container workloads through Quadlet definitions while respecting the CoreOS model of an immutable operating system and containerized services."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Git-defined workload convergence (Priority: P1)

As an operator, I want to define desired workloads in a Git repository so that a
Fedora CoreOS host can continuously converge to that state while respecting the
immutable OS model.

**Why this priority**: This is the primary value of the controller and enables
repeatable, declarative host management.

**Independent Test**: Update the Git repository to add or change a workload and
verify the host converges to the new desired state without manual steps.

**Acceptance Scenarios**:

1. **Given** a valid desired state committed in Git, **When** the controller
   reconciles, **Then** the host reflects the specified Quadlet-managed workloads.
2. **Given** a desired state change in Git, **When** reconciliation runs again,
   **Then** only the intended changes are applied and the host converges.

---

### User Story 2 - Plan and audit before apply (Priority: P2)

As an operator, I want to preview a reconciliation plan and audit the controller's
reasoning so that I can understand and trust changes before they are applied.

**Why this priority**: Safe change review is critical for host-level automation and
builds operator trust.

**Independent Test**: Run the controller in plan mode and verify it reports a
clear, complete plan without applying changes.

**Acceptance Scenarios**:

1. **Given** a desired state change, **When** I run a dry-run, **Then** I receive a
   readable plan and no host changes occur.
2. **Given** a completed reconciliation, **When** I review diagnostics, **Then** I
   can see what changed, why, and whether it succeeded.

---

### User Story 3 - Safe retry and failure handling (Priority: P3)

As an operator, I want explicit failure reporting and safe retries so that partial
failures do not leave the host in an unknown or unsafe state.

**Why this priority**: Reliability and clear recovery paths are required for
production use.

**Independent Test**: Introduce an invalid configuration and verify the controller
surfaces a clear failure and retries do not worsen state.

**Acceptance Scenarios**:

1. **Given** an invalid desired state, **When** reconciliation runs, **Then** the
   controller reports a clear failure and applies no unsupported changes.
2. **Given** a transient failure, **When** reconciliation retries, **Then** it
   converges without duplicating or compounding changes.

### Edge Cases

- Desired state references unsupported mutations on Fedora CoreOS.
- Git repository is temporarily unavailable or unreadable.
- Partial application succeeds but verification fails.
- Observed state diverges due to manual host changes.
- Conflicting or invalid Quadlet definitions are present.
- Desired state requests changes while a prior reconcile is still in progress.
- Host reboot or service restart happens mid-reconciliation.

## Non-Goals *(explicit)*

- Fleet or multi-host management (MVP is single-host only).
- Secret distribution or secret management.
- Full host configuration management beyond Quadlet/systemd/container scope.
- Image registry management, artifact promotion, or deployment pipelines.
- Policy enforcement beyond the declared mutation boundaries.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST treat a Git repository as the source of truth for desired
  state.
- **FR-002**: System MUST represent desired state, observed state, diffs,
  reconciliation plans, and outcomes as explicit data.
- **FR-003**: System MUST reconcile Quadlet-managed workloads on Fedora CoreOS
  within supported mutation boundaries.
- **FR-004**: System MUST provide a dry-run/plan mode that does not apply changes.
- **FR-005**: System MUST expose clear diagnostics and audit records for decisions,
  changes, and failures.
- **FR-006**: Reconciliation MUST be safe to repeat and converge without unintended
  changes.
- **FR-007**: System MUST handle invalid configuration and partial failures
  explicitly, including safe retry behavior.
- **FR-008**: System MUST NOT perform unsupported host mutations outside the
  declared Quadlet/systemd/container scope.
- **FR-009**: System MUST state operational boundaries and non-goals clearly in
  user-facing documentation.
- **FR-010**: System MUST declare reconciliation invariants and verify them
  before apply and after convergence checks.
- **FR-011**: System MUST explicitly define supported mutation boundaries for
  Fedora CoreOS hosts and reject or no-op any requests outside those boundaries.
- **FR-012**: System MUST classify failures (validation, plan, apply, verify,
  transient) and document recovery expectations for each class.

### Reconciliation Invariants

- Desired state and observed state MUST be represented explicitly before any plan
  is computed.
- Plans MUST be derived deterministically from desired and observed state.
- Applying a plan MUST not mutate state outside declared boundaries.
- Reapplying the same desired state MUST yield either no actions or actions that
  are provably idempotent.
- Convergence is reached only when observed state matches desired state within
  the supported boundary definitions.

### Supported Mutation Boundaries (Fedora CoreOS)

- **In scope**: Quadlet unit files, related systemd unit enable/disable state,
  and container lifecycle actions derived from Quadlet definitions.
- **Out of scope**: Base OS mutation, package installation, kernel/system
  configuration, non-Quadlet systemd service changes, and secret management.
- **Rejected operations**: Any requested change that implies host mutation
  outside the Quadlet/systemd/container scope MUST be rejected with an explicit
  reason and no partial apply.

### Failure Semantics and Recovery

- Validation failures MUST prevent apply and return a clear, actionable error.
- Plan computation failures MUST return a reason and leave host state unchanged.
- Apply failures MUST be detected, reported, and followed by a verification step
  that marks the reconcile as failed.
- Verification failures MUST be surfaced explicitly, including which invariant
  or boundary check failed.
- Transient failures SHOULD be retried with backoff, and retries MUST be visible
  in audit records.

### Key Entities *(include if feature involves data)*

- **Desired State**: The Git-defined set of workloads and related settings.
- **Observed State**: The host's actual workloads and relevant runtime state.
- **Reconciliation Plan**: The computed set of actions needed to converge.
- **Workload Definition**: A Quadlet-based unit description and associated
  lifecycle expectations.
- **Audit Record**: A trace of decisions, diffs, actions, and outcomes.
- **Host**: A Fedora CoreOS instance under management.

### Assumptions

- Initial deployment targets a single host while keeping room for future
  multi-host evolution.
- The Git repository is accessible to the controller and contains valid desired
  state definitions.
- Operators prefer safety and clarity over automatic remediation beyond the
  supported scope.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Pure planning and diffing are separated
  from any host changes.
- **Declarative state model**: Desired/observed/plan/outcome are explicit and
  inspectable.
- **Idempotence & convergence**: Reconciliation is repeatable and converges
  deterministically.
- **Explicit effects/failures**: Side effects, assumptions, and failure modes are
  reported explicitly.
- **Observability**: Plans, diffs, actions, outcomes, and diagnostics are visible.
- **Safe defaults**: Risky operations require explicit operator intent.
- **Compatibility**: Changes preserve backward compatibility when feasible and
  document any breaking behavior.
- **Test contract**: Tests cover invariants, external behavior, convergence, and
  failure semantics.
- **Regenerability**: Specs and tests define the contract, enabling safe
  regeneration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 95% of reconciliations converge within two reconciliation cycles for
  valid desired state changes.
- **SC-002**: Operators can produce a dry-run plan in under 30 seconds for a
  standard workload change.
- **SC-003**: 100% of reconciliation attempts produce an operator-visible outcome
  (success or explicit failure).
- **SC-004**: Operators can identify the cause of a failed reconciliation within
  2 minutes using provided diagnostics.
- **SC-005**: Reapplying the same desired state results in zero unintended changes.
