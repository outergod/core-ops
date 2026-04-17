# Feature Specification: Controller State Model and Lifecycle

**Feature Branch**: `015-controller-state-lifecycle`
**Created**: 2026-04-14
**Status**: Draft
**Input**: Adopted from `.agent/spec.md`

---

## Purpose

Define the operator-facing configuration model of CoreOps and introduce a first-class initialization flow that aligns CLI behavior with the controller's persisted state.

This spec formalizes how CoreOps determines:

* which repository it tracks
* which reference within that repository defines desired state
* how this configuration is persisted, inspected, and overridden

---

## Non-Goals

This spec does **not** define:

* source repository layout or schema
* secrets management or integration
* full status UX redesign
* fleet or multi-node behavior
* concurrency/locking semantics

---

## Definitions

* **Controller Configuration**: Operator-supplied, durable configuration persisted by CoreOps.
* **Requested Ref**: A named Git tracking reference — either a branch or a tag. Commit hashes and other arbitrary refs are not valid requested refs.
* **Observed Revision**: A resolved commit hash corresponding to the requested ref.
* **Controller State**: Persisted data including configuration, provenance, and reconciliation metadata (see Specs 004, 006, and 007).

---

## 1. Configuration Model

### 1.1 Authoritative Configuration Fields

The following fields are defined as **operator-facing controller configuration**:

* `desired_state.repository`
* `desired_state.requested_ref`

These fields:

* MUST be persisted in controller state
* MUST be the exclusive source of repository and ref for all reconciliation operations

---

### 1.2 Non-Configuration Fields

The following MUST NOT be treated as configuration:

* `controller.*`
* `desired_state.last_observed_revision`
* `desired_state.last_observed_at`
* `reconciliation.*`

These fields:

* are runtime/provenance metadata
* MUST NOT be user-modifiable via CLI configuration mechanisms

---

### 1.3 Controller Lifecycle States

The controller is always in one of the following states. The current state is determined by the condition of the persisted state file and the value of `reconciliation.status` within it.

---

#### Uninitialized

**Condition**: state file is absent.

* No configuration, provenance, or reconciliation history exists
* All commands MUST fail with a missing-initialization error
* Rollback is not available

**Transition to Initialized**: `core-ops init <repository> <ref>`

---

#### Corrupt

**Condition**: state file is present on disk but is not a valid complete snapshot.

* All commands MUST fail with an explicit error naming the state file path and directing the operator to `core-ops init <repository> <ref> --force`
* This state MUST NOT be treated as Uninitialized; the two MUST produce distinguishable errors
* Rollback is not available

**Transition to Initialized**: `core-ops init <repository> <ref> --force`

---

#### Initialized

**Condition**: valid complete snapshot; `reconciliation.status = never_run`.

* Controller configuration is set; no reconciliation has ever been attempted
* `last_applied_revision` is absent; three-way reconciliation has no baseline
* No retained snapshots exist; rollback is not available
* `apply` and `agent` may execute; their first run establishes the baseline

**Transition to Reconciling**: first `apply` or `agent` execution

---

#### Reconciling

**Condition**: valid complete snapshot; `reconciliation.status = in_progress`; `running = true`; `last_started_at` is set; `last_finished_at` is absent.

* A reconciliation attempt is active and the run lock is held
* `status` reflects that the previous reconciliation outcome is not yet determined

If the controller process terminates abnormally during reconciliation, the state file will remain in this condition at next startup. This is an interrupted reconciliation. Because detecting an abandoned lock is a locking-semantics concern (a non-goal of this spec), the recovery behavior for this case is left to be defined separately.

**Transition to Converged**: reconciliation completes successfully
**Transition to Diverged**: reconciliation fails

---

#### Converged

**Condition**: valid complete snapshot; `reconciliation.status = success`; `last_applied_revision == last_attempted_revision`; both are set; `running = false`.

* The host matches the last successfully applied desired-state revision
* Three-way reconciliation is available using `last_applied_revision` as the baseline
* At least one retained snapshot exists for the current scope; rollback is available

**Transition to Reconciling**: next `apply` or `agent` execution
**Transition to Detached**: successful snapshot rollback
**Transition to Initialized**: `core-ops init <repository> <ref> --force` (see Section 2.4 for history and snapshot retention rules)

---

#### Diverged

**Condition**: valid complete snapshot; `reconciliation.status = failed`; `last_attempted_revision` is set; `running = false`.

* The last reconciliation attempt did not succeed
* `last_attempted_revision` records the revision that failed; `last_applied_revision` records the last known-good revision, which MAY differ
* `attempted_observed_divergence` is set if the attempted revision differed from the most recently observed revision at the time of the attempt
* Three-way reconciliation is available using `last_applied_revision` as the baseline if it is set
* Rollback to retained snapshots is available if any exist for the current scope

**Transition to Reconciling**: next `apply` or `agent` execution
**Transition to Detached**: successful snapshot rollback
**Transition to Initialized**: `core-ops init <repository> <ref> --force`

---

#### Detached

**Condition**: valid complete snapshot; `reconciliation.status = success` or `failed`; a detached flag is set in persisted state.

Entered exclusively via a successful snapshot rollback. The controller has re-applied a previously retained system state and is no longer following the configured `requested_ref`.

* `apply` and `agent` MUST NOT attempt reconciliation against `requested_ref`
* `agent` MUST run, detect the detached state, report it clearly, and exit cleanly without performing reconciliation
* `plan` MUST behave normally: resolve `requested_ref` to its current HEAD and compute the plan using the rolled-back revision as the baseline; plan output MUST clearly indicate the controller is in Detached mode so the operator understands the result as a "what re-attaching would apply" view
* Snapshot rollback is permitted from Detached. A further rollback applies the new snapshot, updates `reconciliation.last_applied_revision`, and leaves the controller Detached with a new currently applied detached revision. All normal eligibility checks apply (scope compatibility, retention window).
* `status` MUST clearly indicate the detached state and the currently applied detached revision
* The detached state persists across restarts until the operator explicitly re-attaches

**Transition to Detached** (further rollback): successful snapshot rollback to a different eligible revision
**Transition to Initialized**: `core-ops init <repository> <ref> --force` — always required from Detached since configuration is already present; MUST clear the detached flag (see Section 2.4 for history and snapshot retention rules)

---

#### Lifecycle Summary

| State         | State file       | `reconciliation.status` | Agent reconciles | Rollback available |
|---------------|------------------|-------------------------|------------------|--------------------|
| Uninitialized | absent           | —                       | No (error)       | No                 |
| Corrupt       | present, invalid | —                       | No (error)       | No                 |
| Initialized   | valid            | `never_run`             | Yes              | No                 |
| Reconciling   | valid            | `in_progress`           | Lock-gated       | No                 |
| Converged     | valid            | `success`               | Yes              | Yes                |
| Diverged      | valid            | `failed`                | Yes              | If snapshots exist |
| Detached      | valid            | `success` or `failed`   | No (paused)      | If snapshots exist |

---

## 2. Initialization Command

### 2.1 Command Definition

CoreOps SHALL introduce:

```
core-ops init <repository> <ref>
```

Where `<repository>` is a local path or Git URL and `<ref>` is a branch name or tag name.

---

### 2.2 Behavior

`init` MUST:

* Validate that `<ref>` is a branch name or tag name resolvable in the given repository
* Reject commit hashes and other non-symbolic refs with a clear error
* Persist:
  * `desired_state.repository`
  * `desired_state.requested_ref`
* Initialize controller configuration if not already present

---

### 2.3 Idempotency

If controller configuration already exists:

* `init` without flags MUST fail with a clear error indicating configuration already exists

Configuration is considered present if the persisted state file is a valid complete snapshot containing non-empty `desired_state.repository` and `desired_state.requested_ref`.

An absent state file MUST be treated as no configuration present.

A corrupt or invalid state file (present on disk but not a valid complete snapshot) MUST NOT be silently treated as absent. CoreOps MUST surface it as an explicit error and direct the operator to run `core-ops init <repository> <ref> --force` to overwrite the corrupt state and reinitialize.

---

### 2.4 Reinitialization

`init` MUST support:

```
core-ops init <repository> <ref> --force
```

Which MUST:

* overwrite `desired_state.repository` and `desired_state.requested_ref`
* clear the detached flag if set

And MAY:

* clear reconciliation history (`reconciliation.*`)
* clear retained snapshots

**Exception**: if `<repository>` and `<ref>` are identical to the currently persisted values (re-attaching without changing tracking configuration), reconciliation history and retained snapshots MUST be preserved. Clearing them in this case would discard the history of a host that is otherwise in a known-good state.

---

## 3. CLI Resolution Model

### 3.1 Repository and Ref Source

The `repository` and `ref` arguments are removed from the following commands:

* `plan`
* `apply`
* `agent`
* `explain`

These commands MUST source `repository` and `ref` exclusively from persisted controller configuration:

* `desired_state.repository`
* `desired_state.requested_ref`

No per-invocation override of repository or ref is supported for these commands. `explain` inspects currently active entities derived from the initialized tracking configuration, not hypothetical revisions.

---

### 3.2 Uninitialized or Corrupt Controller State

If the state file is absent:

* commands MUST fail with a clear, actionable error indicating missing initialization and directing the operator to run `core-ops init`

If the state file is present but corrupt or invalid:

* commands MUST fail with a distinct, actionable error identifying the state file as corrupt and directing the operator to run `core-ops init <repository> <ref> --force` to recover

These two cases MUST produce distinguishable errors. Silently treating a corrupt state file as absent is not permitted.

---

## 4. Ref and Revision Semantics

### 4.1 Requested Ref

* `desired_state.requested_ref` represents:
  * a named tracking reference — a branch or tag — set at initialization time
  * the mutable human-level pointer from which the observed revision is resolved at each reconciliation

---

### 4.2 Observed Revision

* `desired_state.last_observed_revision` represents:
  * the resolved commit corresponding to the requested ref at observation time

---

### 4.3 Resolution Rules

CoreOps MUST:

* resolve `requested_ref` to a commit hash before planning/applying
* record the resolved commit as `last_observed_revision`

---

## 5. Rollback Constraints

### 5.1 Valid Targets

Rollback validity is defined exclusively by CoreOps' retained snapshot state.

A rollback target is valid if and only if:

* a retained snapshot exists for the target revision
* the snapshot's scope is compatible with the current scope
* the snapshot has not expired from the retention window

Git ref reachability is NOT a rollback constraint. A retained snapshot represents a verified working system state regardless of what has subsequently happened to the tracking ref in Git (including force-pushes, rebases, or branch rewrites).

---

### 5.2 Invalid Targets

CoreOps MUST reject rollback attempts where:

* no retained snapshot exists for the target revision
* the retained snapshot's scope is incompatible with the current scope (see below)
* the retained snapshot has expired from the rollback window

#### Scope Compatibility

Each retained snapshot carries a **scope identifier** recording the host identity on which the snapshot was successfully applied. The scope identifier takes the form `host:<hostname>:<machine-id>`, where the hostname is resolved from the `CORE_OPS_HOST` environment variable if set, otherwise from the system hostname at the time of apply. machine-id is taken from `/etc/machine-id`.

A snapshot's scope is compatible with a rollback request only if the snapshot's recorded scope identifier matches the scope identifier of the host performing the rollback.

This constraint exists because retained snapshots encode the normalized state of managed objects — unit files, Quadlet definitions, mount declarations — as they were applied to a specific host. Rolling back a snapshot from a different host would re-apply that host's managed object state to the wrong target, producing undefined behavior.

Scope incompatibility MUST be reported as a distinct, named rejection reason so that operators can distinguish it from a missing or expired snapshot.

---

### 5.3 Rollback Modes

CoreOps supports two rollback mechanisms:

1. Git-driven rollback (canonical)
   - achieved by modifying the tracked repository/ref
   - CoreOps reconciles to the new desired state

2. Snapshot rollback (operational)
   - re-applies a previously recorded system state
   - does not require modification of the source repository

---

### 5.4 Post-Rollback Behavior

After a successful snapshot rollback, CoreOps MUST enter the **Detached** lifecycle state (see Section 1.3).

CoreOps MUST:

* set a detached flag in persisted state before the rollback apply completes, so the state is durable across restarts
* record the rolled-back revision as the current applied revision in `reconciliation.last_applied_revision`
* preserve `desired_state.requested_ref` unchanged — it remains the configured tracking ref but is not followed while detached

While detached, `agent` MUST:

* run on its normal schedule
* detect the detached flag at startup
* emit a clear, observable status indicating the controller is detached and the currently applied detached revision
* exit without performing reconciliation

The operator re-attaches the controller by running `core-ops init <repository> <ref> --force`, which MUST clear the detached flag and resume tracking. `--force` is always required from Detached since configuration is already present. If `<repository>` and `<ref>` are unchanged, reconciliation history and retained snapshots MUST be preserved.

---

## 6. Status Exposure Requirements

CoreOps MUST expose, via `status`:

* `desired_state.repository`
* `desired_state.requested_ref`
* `desired_state.last_observed_revision`
* `reconciliation.last_applied_revision`

The format is not specified in this spec.

---

## 7. Failure Modes

### 7.1 Missing Configuration

If the state file is absent:

* CoreOps MUST fail with a clear, actionable error directing the operator to run `core-ops init`

---

### 7.2 Corrupt State File

If the state file is present on disk but cannot be parsed or is not a valid complete snapshot:

* CoreOps MUST fail with a distinct error identifying the state file path and describing it as corrupt or unreadable
* CoreOps MUST direct the operator to run `core-ops init <repository> <ref> --force` as the explicit recovery path
* CoreOps MUST NOT silently fall through to absent-state behavior

---

### 7.3 Invalid Repository or Ref

If the repository cannot be reached or the ref cannot be resolved:

* CoreOps MUST fail before persisting configuration and provide diagnostic information

If the ref is not a branch or tag (e.g. a bare commit hash or other non-symbolic ref):

* CoreOps MUST reject it with a clear error explaining that only branch and tag names are accepted

---

## 8. Compatibility

This spec builds on Spec 004 and:

* MUST NOT break existing persisted state
* MUST treat existing `desired_state.*` fields as authoritative if present

---

## 9. Rationale

CoreOps already persists tracking configuration implicitly via controller state.

This spec:

* makes that configuration explicit in the operator UX via a dedicated `init` command
* removes `repository` and `ref` arguments from `plan`, `apply`, `agent`, and `explain`, eliminating per-invocation configuration that previously bypassed or duplicated persisted state
* aligns CLI behavior with persisted state so that reconciliation commands are fully driven by initialized controller configuration
* establishes a clear boundary between:
  * operator intent (configuration, set once via `init`)
  * system observation (provenance)
  * reconciliation execution (runtime state)

---

## 10. Future Work

This spec enables:

* improved status UX (human vs machine output)
* source repository specification
* agent behavior standardization
* fleet-level orchestration

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Initialize Controller on a New Host (Priority: P1)

As an operator setting up CoreOps on a new or unconfigured host, I want to run a single `init` command that records the source repository and tracking ref so that all subsequent reconciliation commands operate without requiring those values on every invocation.

**Why this priority**: Without initialization, no reconciliation command can run. This is the prerequisite for all other behavior in this spec.

**Independent Test**: Run `core-ops init <repository> <ref>` on a host with no state file, then run `core-ops plan` and verify it sources repository and ref from persisted state without requiring `--repo` or `--rev` flags.

**Acceptance Scenarios**:

1. **Given** a host with no state file, **When** the operator runs `core-ops init <repository> <branch>`, **Then** `desired_state.repository` and `desired_state.requested_ref` are persisted and `core-ops status` reports the controller as Initialized.
2. **Given** a host with valid existing configuration, **When** the operator runs `core-ops init <repository> <ref>` without `--force`, **Then** the command fails with a clear error indicating configuration already exists.
3. **Given** a host with a corrupt state file, **When** the operator runs `core-ops init <repository> <ref>` without `--force`, **Then** the command fails with a distinct error identifying the file as corrupt and directing the operator to use `--force`.
4. **Given** `core-ops init` is invoked with a bare commit hash as `<ref>`, **When** the command runs, **Then** it is rejected with a clear error stating only branch and tag names are accepted.

---

### User Story 2 - Survive a Snapshot Rollback Without Losing Recovered State (Priority: P2)

As an operator who has performed a snapshot rollback to recover a broken host, I want the controller to enter detached mode so that the next scheduled agent run does not silently overwrite my recovered state.

**Why this priority**: Without detached mode, rollback under an unattended systemd timer is transient — the next timer tick re-applies the tracking branch HEAD and undoes the recovery. This is the core value of the Detached lifecycle state.

**Independent Test**: Perform a snapshot rollback, then trigger the agent timer and verify the agent exits without reconciling, reports the Detached state and currently applied detached revision, and does not modify host-managed units.

**Acceptance Scenarios**:

1. **Given** the controller is in Converged state, **When** a snapshot rollback completes successfully, **Then** the controller enters Detached state with the detached flag set in persisted state and `reconciliation.last_applied_revision` updated to the rolled-back revision.
2. **Given** the controller is in Detached state, **When** the systemd agent timer fires, **Then** `agent` runs, emits an observable status message naming the detached state and currently applied detached revision, and exits without modifying managed units.
3. **Given** the controller is in Detached state, **When** the operator runs a further snapshot rollback to a different eligible revision, **Then** the controller remains Detached with the new currently applied detached revision.
4. **Given** the controller is in Detached state, **When** the operator runs `core-ops init <repository> <ref> --force` with the same repository and ref, **Then** the detached flag is cleared, reconciliation resumes, and reconciliation history and retained snapshots are preserved.

---

### User Story 3 - Inspect What Re-Attaching Would Apply While Detached (Priority: P3)

As an operator in Detached state deciding whether to re-attach, I want `plan` to show me what reconciliation would apply if I resumed tracking the configured ref so that I can make an informed decision before committing.

**Why this priority**: The Detached state is meant to give the operator control, not to obscure forward state. A readable plan is the minimum viable inspection surface for this decision.

**Independent Test**: With the controller in Detached state, run `core-ops plan` and verify the output clearly signals Detached mode, uses the currently applied detached revision as the baseline, and resolves `requested_ref` to its current HEAD as the target.

**Acceptance Scenarios**:

1. **Given** the controller is in Detached state, **When** the operator runs `core-ops plan`, **Then** the plan output clearly indicates Detached mode, computes the delta from the currently applied detached revision to the current `requested_ref` HEAD, and does not trigger reconciliation.

---

### User Story 4 - Recover from a Corrupt State File (Priority: P4)

As an operator whose state file has been corrupted (interrupted write, manual edit, disk error), I want a distinct error with an exact recovery command so that I can restore the controller without diagnosing the file format manually.

**Why this priority**: Silently treating a corrupt file as absent would mask data loss. A distinct, named error with recovery instructions is the minimum viable safety surface.

**Independent Test**: Replace the state file with malformed JSON and run any reconciliation command. Verify the error names the file path, describes it as corrupt, and provides `core-ops init <repository> <ref> --force` as the recovery command. Verify the error is visibly different from the missing-initialization error.

**Acceptance Scenarios**:

1. **Given** the state file exists but contains malformed content, **When** any CoreOps command runs, **Then** the error names the state file path, describes it as corrupt or unreadable, and directs the operator to run `core-ops init <repository> <ref> --force`.
2. **Given** the state file is absent, **When** any CoreOps command runs, **Then** the error is visibly distinct from the corrupt-state error and directs the operator to run `core-ops init`.

---

### Edge Cases

- `init --force` on a host in Detached state with the same repository and ref must preserve reconciliation history and retained snapshots while clearing the detached flag.
- `init --force` on a host in Detached state with a different repository or ref may clear reconciliation history and retained snapshots.
- The controller crashes mid-reconciliation, leaving `reconciliation.status = in_progress` in the state file. The recovery behavior for this interrupted state is out of scope for this spec.
- `init` on a host where the state file directory does not yet exist must create the directory.
- `init` with a ref that resolves correctly but is neither a branch nor a tag (e.g. `HEAD`) must be rejected.
- A rollback target that is scope-compatible and within the retention window but whose snapshot is otherwise incomplete must be treated as missing.

---

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CoreOps MUST introduce a `core-ops init <repository> <ref>` command that persists `desired_state.repository` and `desired_state.requested_ref` to the canonical state file.
- **FR-002**: `init` MUST validate that `<ref>` resolves as a branch or tag in the given repository; commit hashes and other non-symbolic refs MUST be rejected with a clear error.
- **FR-003**: `init` without `--force` MUST fail with a clear error if a valid complete snapshot containing non-empty `desired_state.repository` and `desired_state.requested_ref` already exists.
- **FR-004**: `init` without `--force` MUST fail with a distinct error (not the same as missing-initialization) if the state file is present but corrupt.
- **FR-005**: `init --force` MUST overwrite `desired_state.repository` and `desired_state.requested_ref` and clear the detached flag if set.
- **FR-006**: `init --force` MUST preserve reconciliation history and retained snapshots when `<repository>` and `<ref>` are identical to the currently persisted values.
- **FR-007**: `plan`, `apply`, `agent`, and `explain` MUST source `repository` and `ref` exclusively from persisted controller configuration; per-invocation `--repo` and `--rev` arguments are removed from these commands.
- **FR-008**: Commands MUST produce distinct, actionable errors for an absent state file versus a corrupt state file; the two cases MUST NOT produce the same error.
- **FR-009**: A corrupt state file MUST NOT be silently treated as absent by any command.
- **FR-010**: A successful snapshot rollback MUST transition the controller to Detached state by setting a detached flag in persisted state before the rollback apply completes.
- **FR-011**: In Detached state, `agent` MUST run on schedule, detect the detached flag, emit an observable status naming the currently applied detached revision, and exit without performing reconciliation.
- **FR-012**: In Detached state, `plan` MUST resolve `requested_ref` to its current HEAD, compute the plan using the currently applied detached revision as the baseline, and clearly indicate Detached mode in output.
- **FR-013**: Snapshot rollback MUST be permitted from Detached state; the controller remains Detached after a further rollback with the new currently applied detached revision.
- **FR-014**: Rollback validity MUST be determined exclusively by retained snapshot eligibility: snapshot existence, scope compatibility (`host:<hostname>:<machine-id>`), and retention window; Git ref reachability MUST NOT be a rollback constraint.
- **FR-015**: Scope incompatibility MUST be reported as a distinct named rejection reason distinguishable from missing snapshot and expired snapshot.
- **FR-016**: `status` MUST expose `desired_state.repository`, `desired_state.requested_ref`, `desired_state.last_observed_revision`, and `reconciliation.last_applied_revision`.
- **FR-017**: `status` MUST clearly indicate when the controller is in Detached state and report the currently applied detached revision.
- **FR-018**: CoreOps MUST NOT break existing persisted state; existing `desired_state.*` fields MUST be treated as authoritative if present.

### Key Entities

- **Controller Configuration**: The operator-supplied durable values `desired_state.repository` and `desired_state.requested_ref`, set via `init` and exclusively used as the source of repository and ref for all reconciliation operations.
- **Lifecycle State**: The named operational state of the controller at any given time — one of Uninitialized, Corrupt, Initialized, Reconciling, Converged, Diverged, or Detached — determined by the state file condition and `reconciliation.status`.
- **Detached Flag**: A persisted boolean in controller state indicating the controller is not following `requested_ref`; set on snapshot rollback, cleared by `init --force`.
- **Requested Ref**: A branch or tag name set at initialization time; the mutable human-level pointer resolved to an immutable commit hash at each reconciliation.
- **Observed Revision**: The immutable commit hash produced by resolving `requested_ref` at observation time; recorded as `desired_state.last_observed_revision`.
- **Retained Snapshot**: A normalized record of a successfully applied system state for a given scope and revision, eligible for snapshot rollback subject to scope compatibility and retention window constraints.

---

## Verification Guidance *(mandatory)*

### Observable Behaviors

- `core-ops init <repository> <ref>` persists repository and ref to the state file and reports success
- `core-ops init` without `--force` fails when valid configuration exists
- `core-ops init` on a corrupt state file fails with a distinct corrupt-state error
- `core-ops init --force` clears the detached flag and resumes tracking
- Any reconciliation command fails with a missing-initialization error when the state file is absent
- Any reconciliation command fails with a distinct corrupt-state error when the state file is present but invalid
- `agent` in Detached state exits without modifying managed units and emits a detached status message
- `plan` in Detached state produces output clearly indicating Detached mode
- Snapshot rollback sets the detached flag before completing
- Further rollback from Detached updates the currently applied detached revision and preserves Detached state
- `status` reports the currently applied detached revision and detached flag when in Detached state

### Invariants

- `desired_state.repository` and `desired_state.requested_ref` are never sourced from CLI invocation arguments after this spec; only from persisted state
- Absent and corrupt state files always produce visibly different errors
- The detached flag survives controller restart
- Re-attaching with unchanged repository and ref always preserves reconciliation history and retained snapshots
- Git ref reachability never affects rollback eligibility

### Idempotency Expectations

- Running `core-ops init <repository> <ref>` twice without `--force` fails on the second invocation without modifying state
- Running `core-ops init <repository> <ref> --force` twice with the same values preserves history on both invocations
- `agent` invoked repeatedly in Detached state exits cleanly each time without side effects

### Failure Modes

- Absent state file: missing-initialization error naming `core-ops init` as the recovery action
- Corrupt state file: corrupt-state error naming the file path and `core-ops init <repository> <ref> --force` as the recovery action
- Non-symbolic ref supplied to `init`: rejection error stating only branch and tag names are accepted
- Unresolvable repository or ref: failure before persisting configuration, with diagnostic output
- Rollback to missing snapshot: named rejection `MissingSnapshot`
- Rollback to incompatible scope: named rejection `IncompatibleScope`
- Rollback to expired snapshot: named rejection `Expired`

### Upgrade Considerations

- Existing state files without a detached flag MUST be treated as not detached
- Existing `desired_state.*` fields MUST be treated as authoritative without requiring re-initialization
- The schema version mechanism from Spec 004 applies; this spec MUST NOT introduce changes that invalidate existing valid snapshots

### Required Scenario Classes

- Fresh initialization on a host with no prior state
- Reinitialization with `--force` using same repository and ref (history preserved)
- Reinitialization with `--force` using different repository or ref (history MAY be cleared)
- Snapshot rollback from Converged and from Diverged transitioning to Detached
- Further rollback from Detached
- `agent` execution in Detached state (no reconciliation, clear status)
- `plan` execution in Detached state (Detached context indicated)
- Re-attachment from Detached with same and different repository/ref
- Absent state file producing missing-initialization error
- Corrupt state file producing distinct corrupt-state error

---

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Lifecycle state determination, rollback eligibility evaluation, configuration validation, and detached flag detection are pure logic over data structures; state file I/O, Git ref resolution, CLI argument parsing, and systemd interaction remain in boundary layers.
- **Declarative state model**: Lifecycle states are defined as named conditions with explicit entry predicates and invariants; controller configuration is declared data with a clear ownership boundary.
- **Idempotence & convergence**: `init --force` with unchanged repository and ref preserves history and produces a stable result on repeated invocation; `agent` in Detached state exits cleanly each run without accumulating side effects.
- **Explicit effects/failures**: Absent and corrupt state files produce distinct named errors; Detached state is an explicit, observable, named lifecycle state rather than an implicit condition; all failure modes carry recovery paths.
- **Observability**: `status` exposes the fields required to determine lifecycle state; `agent` reports detached status clearly; `plan` in Detached mode signals context; rollback rejections carry named eligibility reasons.
- **Provenance & traceability**: `desired_state.repository` and `desired_state.requested_ref` are the authoritative configuration inputs, set once via `init` and reported via `status`; lifecycle state is machine-readable.
- **Safe defaults**: `init` without `--force` fails rather than overwriting; Detached state pauses reconciliation rather than silently forwarding; corrupt state is an explicit error, not a silent fallback; rollback validity is defined by retained snapshots, not Git reachability.
- **Compatibility**: Existing `desired_state.*` fields are authoritative without re-initialization; existing state files without a detached flag are treated as not detached; this spec builds on Spec 004 without breaking valid existing snapshots.
- **Release version policy**: Removing `--repo`/`--rev` from `plan`, `apply`, `agent`, and `explain` is a breaking CLI change requiring a major version increment; adding `init`, detached mode, and the detached flag requires at minimum a minor version increment; all changes update `Cargo.toml` accordingly.
- **Release intent artifact**: `changes/015-controller-state-lifecycle.md` must declare the appropriate SemVer intent (`major` for the CLI breaking changes).
- **Changelog discipline**: All externally visible changes — the `init` command, removed `--repo`/`--rev` flags, detached mode behavior, and corrupt-state error distinction — must be documented in `CHANGELOG.md` before the work is considered complete.
- **Test contract**: Tests must cover: `init` success and failure paths; lifecycle state transitions; absent vs corrupt state file error distinction; Detached mode agent and plan behavior; rollback eligibility and rejection reasons; re-attachment with same and different repository/ref. `cargo test` and `cargo clippy --all-targets -- -D warnings` must pass.
- **Regenerability**: The lifecycle state machine, named invariants, and behavioral test surface provide sufficient specification to regenerate the implementation; scenario classes map directly to integration test cases.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of fresh-host initialization scenarios covered by acceptance tests, running `core-ops init <repository> <ref>` followed by any reconciliation command succeeds without requiring per-invocation `--repo` or `--rev` arguments.
- **SC-002**: In 100% of scenarios where the state file is absent or corrupt, the resulting error message names the recovery action unambiguously and the two cases produce visibly distinguishable output.
- **SC-003**: In 100% of snapshot rollback scenarios, the controller enters Detached state and subsequent `agent` invocations do not apply changes to managed units until the operator explicitly re-attaches.
- **SC-004**: In 100% of re-attachment scenarios where repository and ref are unchanged, reconciliation history and retained snapshots are present after re-attachment.
- **SC-005**: In 100% of rollback rejection scenarios, the rejection reason is one of `MissingSnapshot`, `IncompatibleScope`, or `Expired`, with no rejections based on Git ref reachability.
- **SC-006**: All externally visible behavior changes introduced by this spec are documented in `CHANGELOG.md` and declared in `changes/015-controller-state-lifecycle.md` before the feature is merged.
