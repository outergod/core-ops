# Feature Specification: Config Change Restart Fidelity

**Feature Branch**: `014-config-restart-fidelity`
**Created**: 2026-04-12
**Status**: Draft
**Input**: "Config Change Restart Reporting Diverges From Actual Apply" from `docs/follow-ups.md`.

## Problem Statement

When a `ConfigFile`-typed quadlet changes during apply, the operator-facing report says the consuming
container `restarted`, but the service never actually restarts. Two independent bugs combine to produce
this:

1. **Executable planner gap** (`src/core/planner.rs` → `actions_for_diff`): for `QuadletType::ConfigFile`
   diffs, `manage_unit = false` and `reload_systemd = false`, so only `WriteQuadlet` is scheduled.
   No `RestartUnit` is emitted for containers/services that list the config file as a dependency. The
   dependency graph exists and is queryable via `dependent_refs`, but the `plan()` path never consults
   it for config-file changes.

2. **Report sourcing mismatch** (`src/cli/report.rs`): terminal `restarted` status in human/machine
   output is derived from `DeterministicActionClass::Restart` — the declarative plan classification —
   not from actual `ExecutionEvent` records produced during apply. So the report says `restarted` even
   when no `RestartUnit` action was executed.

## Clarifications

### Session 2026-04-12

- Q: When a config file is deleted (`DiffKind::Remove`), should dependent containers be restarted? → A: Yes — schedule `RestartUnit` for dependent containers on removal, same as Change, to surface the missing-config failure immediately.
- Q: Should SC-002 (live-host restart after config change) be covered by an automated e2e verification scenario or manual/operational validation only? → A: Automated e2e verification scenario — add a scenario to the verification harness that applies a config change and asserts the consuming service restart timestamp advances.
- Q: When a config file is added (`DiffKind::Add`) and a dependent container is already running, should the container be restarted? → A: Yes — restart if the container is present in observed state (already running); skip restart only when no prior observed state exists for the consuming container.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Config change triggers actual service restart (Priority: P1)

An operator runs `core-ops apply` on a host where a container's config file has changed. After apply
completes, the service has been restarted and is running with the new configuration. Querying
`systemctl status` shows a post-apply start timestamp newer than before the apply.

**Why this priority**: This is the correctness regression. Without it, applying config changes silently
leaves services in a stale state, which is the core operator safety guarantee the tool must uphold.

**Independent Test**: Can be tested with a unit integration test that builds a desired state containing
a container with a `ConfigFile` dependency, drives a `Change` diff for the config file, and asserts
the resulting `ReconciliationPlan.actions` contains a `RestartUnit` action for the consuming container.

**Acceptance Scenarios**:

1. **Given** a container workload with a `ConfigFile` dependency, **When** the config file content
   changes and `plan()` is invoked, **Then** the produced `ReconciliationPlan.actions` includes a
   `RestartUnit` action for the consuming container after `WriteQuadlet`.
2. **Given** a config file change that affects multiple dependent containers, **When** the plan is
   produced, **Then** a `RestartUnit` action is emitted for each dependent container — not just the
   first.
3. **Given** a config file change where no workload lists it as a dependency, **When** the plan is
   produced, **Then** no spurious `RestartUnit` actions are added.
4. **Given** a config file that is added for the first time and the dependent container has no prior
   observed state (not yet running), **When** the plan is produced, **Then** no `RestartUnit` is
   scheduled for that container — its own `StartUnit` action is sufficient.
5. **Given** a config file that is added for the first time and the dependent container is already
   running (present in observed state), **When** the plan is produced, **Then** a `RestartUnit` is
   scheduled for the container so it picks up the new config.

---

### User Story 2 — Apply report reflects actual execution, not planned classification (Priority: P2)

An operator reads the terminal output of `core-ops apply`. A line that says `restarted` for a container
means that container was actually restarted during this apply run — not merely that the deterministic
plan classified it as a restart candidate.

**Why this priority**: Operator trust depends on the apply output being a faithful record of what
happened, not a speculation about what should have happened. Overstating effects is worse than silence:
it masks stale state behind a false assurance.

**Independent Test**: Can be tested by verifying that when no `RestartUnit` action is scheduled for a
container, the terminal apply output does not emit `restarted` for that container.

**Acceptance Scenarios**:

1. **Given** a container whose deterministic plan class is `Restart` but for which no `RestartUnit`
   was present in the executed action list, **When** the apply report is rendered, **Then** the
   container is not shown as `restarted` in terminal output.
2. **Given** a container for which a `RestartUnit` action was executed and succeeded, **When** the
   apply report is rendered, **Then** the container is shown as `restarted` with the
   execution-sourced status.
3. **Given** a container for which a `RestartUnit` action was executed but failed, **When** the apply
   report is rendered, **Then** the container is shown as failed (not `restarted`), and the error is
   surfaced.

---

### User Story 3 — Regression test for config-change → restart chain (Priority: P3)

A contributor modifying planner logic for config files has a focused integration test that detects the
regression observed on `ulthar`: a config file change for a container-backed service must produce a
`RestartUnit` action in the executable plan.

**Why this priority**: Without a test, the fix for P1 can silently regress. The original bug was
undetected in CI for the same reason.

**Independent Test**: Can be verified by temporarily reverting the P1 fix and confirming the test
fails, then re-applying the fix and confirming it passes.

**Acceptance Scenarios**:

1. **Given** the regression test from P1, **When** P1's fix is reverted, **Then** the test fails,
   proving the test is effective.
2. **Given** the regression test, **When** it runs in CI, **Then** it completes without requiring a
   live systemd host (pure unit test against planner logic).

---

### Edge Cases

- What happens when a config file is deleted (`DiffKind::Remove`)? Dependent containers receive a
  `RestartUnit` action — same as for a `Change` diff. The restart surfaces the missing-config failure
  immediately rather than leaving the service silently running against an absent file.
- What if a config file is added for the first time and its dependent container is already running?
  The container receives a `RestartUnit` so it picks up the new config — observed state is the
  discriminator, not diff kind alone.
- What if a config file is simultaneously a dependency of both a container and a socket? Both consuming
  workloads should receive a `RestartUnit` action.
- What if the same container has multiple changed config file dependencies in one apply? Deduplication
  should ensure only one `RestartUnit` per consuming workload is emitted.
- What ordering is required? `WriteQuadlet` for the config file must precede `RestartUnit` for dependent
  containers. The existing `order_for_type` already places `ConfigFile` at order 0 (before `Container`
  at 5), so action ordering is preserved by the existing ordering scheme.
- What if the container itself also changed independently in the same plan? The container's own diff
  already emits a `RestartUnit`; the config-file-sourced restart must not create a duplicate.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `plan()` in `src/core/planner.rs` MUST, when processing a `DiffKind::Change`,
  `DiffKind::Remove`, or `DiffKind::Add` for a `QuadletType::ConfigFile` object, look up all
  workloads that list the config file as a dependency and emit a `RestartUnit` action for each
  workload that is present in the observed state (already running). For `Add`: only already-running
  consumers are restarted — a container with no prior observed state will be started fresh by its
  own diff actions and needs no separate restart. For `Remove`: forcing a restart on still-running
  consumers surfaces the missing-config failure immediately rather than allowing silent stale
  operation.
- **FR-002**: The dependency lookup MUST use the existing `dependent_refs` / semantic dependency graph
  infrastructure, not a bespoke name-matching heuristic.
- **FR-003**: The resulting `RestartUnit` actions for config-file-dependent restarts MUST be ordered
  after `WriteQuadlet` for the config file and consistent with the existing ordering scheme for
  container restarts.
- **FR-004**: Deduplication MUST prevent double `RestartUnit` actions when the consuming container is
  also independently changed in the same plan.
- **FR-005**: Apply reporting in `src/cli/report.rs` MUST derive terminal `restarted` / `failed` status
  for each workload from the actual `ExecutionEvent` records in the apply output, not solely from the
  `DeterministicActionClass` of the planned object.
- **FR-006**: A unit integration test MUST assert that a `ConfigFile` change for a container with a
  declared dependency produces a `RestartUnit` action for that container in the executable plan.
- **FR-007**: The fix MUST NOT affect the deterministic reconciliation plan
  (`DeterministicReconciliationPlan`) classification logic — that path already correctly classifies
  dependent containers as `Restart` and is not the source of the bug.

### Key Entities

- **Executable plan** (`ReconciliationPlan.actions`): The ordered list of `PlanAction` values actually
  executed by `src/io/apply.rs`. This is the authoritative source for what apply does.
- **Deterministic plan** (`DeterministicReconciliationPlan`): The human/machine-readable classification
  of convergence actions derived from the semantic dependency graph. Used for `plan` output and apply
  progress rendering. Not executed directly.
- **Dependent workload**: A container or service whose quadlet file contains a reference to a
  `ConfigFile` object, modelled as a `dependency_refs` edge in the desired state graph.
- **Report sourcing**: Which data structure the apply renderer reads to determine final object state for
  terminal output. Currently: deterministic plan class. Required after fix: execution event records.

## Verification Guidance *(mandatory for features that participate in the verification workflow)*

### Observable Behaviors

- A changed config file whose consuming container has a dependency edge produces a `RestartUnit` action
  in the executable plan.
- Apply terminal output for a container shows `restarted` only when a `RestartUnit` execution event
  with a success outcome is present in the apply output.

### Invariants

- Every `RestartUnit` action in the executable plan corresponds to an actual systemd restart attempt
  during apply.
- The deterministic plan's `Restart` classification for a container is unaffected by this fix.

### Idempotency Expectations

- Re-applying the same config file content produces no diff and no `RestartUnit` — idempotence is
  unchanged.

### Failure Modes

- A `RestartUnit` execution event with a failure outcome surfaces as a failed workload in apply output,
  not silently consumed.

### Upgrade Considerations

- Hosts that previously applied a config change without triggering a restart will, after this fix, see
  a real restart on the next apply that includes a config file change. This is the intended correction,
  not a regression.

### Required Scenario Classes

- Config-file change with a single dependent container: produces one `RestartUnit` (unit test).
- Config-file change with no dependents: produces no `RestartUnit` (unit test).
- Config-file change plus independent container change in same plan: no duplicate `RestartUnit` (unit test).
- Config-file-only change on a live host: `ActiveEnterTimestamp` of the consuming service advances
  after apply completes (e2e verification harness scenario).
- Config-file deletion with a dependent container: consuming service receives `RestartUnit` and the
  apply report reflects the restart attempt (unit test + e2e).
- Config-file addition with an already-running dependent container: container receives `RestartUnit`
  (unit test).
- Config-file addition with no prior observed state for dependent container: no `RestartUnit`
  scheduled (unit test).

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: The planner fix is pure logic in `plan()` — it reads desired
  state and the dependency graph and produces an updated action list. No new I/O. Apply remains the
  sole executor of `RestartUnit` actions.
- **Declarative state model**: The fix brings the executable plan into alignment with intent already
  expressed in the semantic dependency graph. Config file → container dependency edges are already
  declared in the repo structure; the planner was not honouring them in the executable path.
- **Idempotence & convergence**: A re-apply with unchanged config produces no diff and no restart.
  The fix does not change idempotence guarantees.
- **Explicit effects/failures**: After the fix, every `RestartUnit` in the apply log has a
  corresponding executable action. The report-sourcing fix ensures failures surface as failures, not
  as ghost `restarted` lines.
- **Observability**: Apply output becomes a faithful log of executed effects. Operators can verify
  whether a restart was attempted by reading the action list or the apply report.
- **Provenance & traceability**: No change to revision tracking or provenance records; the fix is
  scoped to action generation and report rendering.
- **Safe defaults**: Scheduling a restart when a config file changes is strictly safer than not
  scheduling one. Stale service state is the operational hazard this spec eliminates.
- **Compatibility**: The `core-ops plan` output is unaffected — it already shows the correct `Restart`
  classification for dependent containers. Only the executable path and the apply report change.
- **Release version policy**: This is a bug fix with operational safety impact; a patch version bump
  is appropriate.
- **Release intent artifact**: A `changes/014-config-restart-fidelity.md` fragment with
  `release_intent: patch` is required before merge.
- **Changelog discipline**: `CHANGELOG.md` must be updated with a `Fixed` entry covering both the
  executable planner gap and the report sourcing correction.
- **Test contract**: The regression test (FR-006) must be a pure Rust unit/integration test
  exercisable via `cargo test` with no live systemd dependency. `cargo clippy --all-targets -- -D
  warnings` must pass without new warnings.
- **Regenerability**: The spec fully describes the expected planner and report contract, enabling safe
  future regeneration of tests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A `ConfigFile` change for a container with a declared dependency produces a `RestartUnit`
  action for that container in `ReconciliationPlan.actions`.
- **SC-002**: Applying a config-file-only change on a live host results in the consuming service being
  restarted, with a post-apply `ActiveEnterTimestamp` newer than the pre-apply value. This criterion
  is validated by an automated e2e verification harness scenario.
- **SC-003**: Terminal apply output shows `restarted` only when a `RestartUnit` action was actually
  executed and succeeded for that workload.
- **SC-004**: The regression test fails when the P1 fix is reverted and passes with it in place.
- **SC-005**: No double `RestartUnit` actions appear in plans where the consuming container is also
  independently changed in the same plan.
