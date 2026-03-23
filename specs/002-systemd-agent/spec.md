# Feature Specification: Systemd-Managed Host Agent

**Feature Branch**: `[002-systemd-agent]`  
**Created**: 2026-03-19  
**Status**: Draft  
**Input**: User description: "Specify the next iteration of the Fedora CoreOS Quadlet GitOps controller as an automated host agent. Goals: - install and operate the controller as a systemd-managed service or timer on a host - validate and rely on journald-based operational observability when running under systemd - extend reconciliation support beyond container Quadlets to include socket and volume Quadlet artifacts - define lifecycle, ordering, and verification behavior for these supported artifact types - preserve the current principles of native system primitives, explicit failure, idempotence, and observability Requirements: - the controller must be able to run unattended on a host through systemd automation - journald should be the default operational audit sink under service execution - socket artifacts must be treated according to their distinct lifecycle semantics - volume artifacts should be supported as first-class reconciled objects - mounting existing host config directories into containers may be supported where it fits naturally within Quadlet definitions - avoid expanding into generic host configuration management Non-goals for this iteration: - host templating or reusable manifest parameterization across multiple hosts - secret distribution - arbitrary environment file management as first-class managed objects - fleet orchestration"

## Clarifications

### Session 2026-03-19
- Q: Which systemd automation mode should the agent ship for unattended runs? → A: Provide both a oneshot service and a timer (timer triggers service)
- Q: What ordering should apply across volume/container/socket artifacts? → A: Volume → Container → Socket ordering
- Q: What verification approach should be used per artifact type? → A: Verify by systemd unit state (active for container/socket; loaded for volume). Do not enable/disable generated units.
- Q: Where are socket artifacts installed? → A: Socket artifacts are native systemd units stored in the system unit directory (e.g., /etc/systemd/system), not in the Quadlet directory.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automated host agent runs unattended (Priority: P1)

As an operator, I want the controller installed as a systemd-managed service or
 timer so that reconciliation runs unattended on a Fedora CoreOS host.

**Why this priority**: Unattended host automation is the core value of this
 iteration and enables reliable day-2 operations.

**Independent Test**: Install the unit files, enable the timer or service, and
 verify the agent executes on schedule without manual invocation.

**Acceptance Scenarios**:

1. **Given** the systemd unit files are installed, **When** the timer fires (or
   the service starts), **Then** reconciliation runs without user interaction.
2. **Given** the agent runs under systemd, **When** it completes a run, **Then**
   operational logs are visible in journald.

---

### User Story 2 - Reconcile containers, sockets, and volumes (Priority: P2)

As an operator, I want the controller to reconcile container, socket, and volume
 Quadlet artifacts so that all supported workload types converge together.

**Why this priority**: Broadening Quadlet support is the key functional expansion
 beyond the current container-only scope.

**Independent Test**: Place container, socket, and volume Quadlet files in the
 repository and verify they are reconciled in a single run.

**Acceptance Scenarios**:

1. **Given** container, socket, and volume Quadlet definitions, **When**
   reconciliation runs, **Then** each artifact type is created/updated or
   removed according to the desired state.
2. **Given** a socket definition that depends on a container service, **When**
   reconciliation runs, **Then** ordering rules ensure a stable, converged state.

---

### User Story 3 - Explicit verification and observability (Priority: P3)

As an operator, I want clear verification behavior and journal-based observability
 so that I can validate changes and diagnose failures.

**Why this priority**: Observability and verification prevent silent failures and
 maintain trust in unattended automation.

**Independent Test**: Introduce a failing Quadlet definition and confirm that the
 agent logs the failure and marks the run as failed without partial success.

**Acceptance Scenarios**:

1. **Given** an invalid Quadlet definition, **When** the agent runs, **Then** it
   reports a failure in journald with a clear reason.
2. **Given** a successful run, **When** I inspect journald logs, **Then** I can
   identify the plan, actions, and outcome for that run.

### Edge Cases

- Timer overlaps a previous run (must avoid concurrent reconciliation).
- Socket artifacts reference services that are missing or invalid.
- Volume artifacts exist on disk but are removed from desired state.
- Host reboots during a reconcile run.
- Journald unavailable or log storage is full.
- Git repository temporarily unavailable.
- Quadlet files contain unsupported extensions.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The controller MUST run unattended via systemd service or timer
  execution on a Fedora CoreOS host. The distribution MUST include both a
  oneshot service and a timer that triggers it.
- **FR-002**: The controller MUST emit operational audit events to journald when
  running under systemd.
- **FR-003**: The controller MUST reconcile container and volume Quadlet
  artifacts (stored in the Quadlet directory) and socket artifacts as native
  systemd units (stored in the system unit directory).
- **FR-004**: The controller MUST define and enforce lifecycle ordering for
  socket units and volume artifacts relative to containers and services, using a
  Volume → Container → Socket ordering model.
- **FR-005**: The controller MUST define explicit verification behavior for each
  supported artifact type (container, socket, volume) using systemd unit state
  checks. Container and socket units MUST report active; volume artifacts MUST
  report a loaded unit state when applicable. The controller MUST NOT run
  enable/disable for generated units; enablement remains driven by Quadlet
  [Install] semantics.
- **FR-006**: The controller MUST remain limited to Quadlet/systemd/container
  scope and MUST NOT expand into generic host configuration management.
- **FR-007**: Mounting existing host config directories into containers MAY be
  supported when specified in Quadlet definitions; the controller MUST NOT manage
  those directories as independent objects.
- **FR-008**: The controller MUST ensure reconciliation runs are idempotent and
  safe to repeat.
- **FR-009**: The controller MUST surface failures explicitly in operator-visible
  diagnostics and logs.
- **FR-010**: The controller MUST prevent overlapping reconcile runs (single-run
  lock semantics).

### Key Entities *(include if feature involves data)*

- **Host Agent Run**: A single scheduled execution with plan, actions, and outcome.
- **Quadlet Artifact**: A desired-state object (container, socket, volume).
- **Reconciliation Plan**: The ordered actions required to converge.
- **Verification Result**: Outcome of post-apply checks per artifact type.
- **Audit Event**: Structured operational log entry emitted to journald.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Planning and verification remain pure;
  systemd/Git interactions are isolated to adapters.
- **Declarative state model**: Desired/observed/plan/outcome remain explicit data.
- **Idempotence & convergence**: Repeated runs converge with no unintended change.
- **Explicit effects/failures**: Side effects and failure modes are logged and
  returned explicitly.
- **Observability**: Journald provides default operational audit output; rich
  reports remain operator-visible.
- **Safe defaults**: Unattended runs still require explicit desired state and
  fail safely on invalid inputs.
- **Compatibility**: No fleet or host config expansion; existing semantics stay
  stable for container Quadlets.
- **Test contract**: Tests cover artifact ordering, lifecycle, verification, and
  unattended execution behavior.
- **Regenerability**: Spec and tests define the contract for safe regeneration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 95% of scheduled runs complete within 2 minutes on a host with up
  to 50 Quadlet artifacts.
- **SC-002**: 100% of runs emit a journald audit event with plan summary and
  outcome status.
- **SC-003**: 100% of failed runs emit a journald audit event containing run_id,
  plan summary, failed artifact list, and failure reason.
- **SC-004**: Reapplying the same desired state results in zero unintended
  changes across three consecutive scheduled runs.
