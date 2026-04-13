# Implementation Plan: Config Change Restart Fidelity

**Branch**: `014-config-restart-fidelity` | **Date**: 2026-04-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/014-config-restart-fidelity/spec.md`

## Summary

Two bugs combine to break config-file-change restarts: (1) `plan()` in
`src/core/planner.rs` emits no `RestartUnit` for containers that depend on a
changed/added/removed `ConfigFile` quadlet; (2) `build_apply_output` in
`src/cli/report.rs` synthesises terminal state from the deterministic plan
classification rather than from actual execution — so the apply report says
`restarted` even when no restart occurred. The fix adds a dependent-restart
pass after the main diff loop in `plan()` (using `dependency_refs_for_workload_state`
from `src/core/evaluate.rs` to locate consumers) and validates that post-apply
verification surfaces restart failures correctly.

## Technical Context

**Language/Version**: Rust 2021 edition  
**Primary Dependencies**: clap 4, serde, serde_json, serde_yaml, miette  
**Storage**: N/A  
**Testing**: `cargo test`, `cargo clippy --all-targets -- -D warnings`  
**Target Platform**: Linux (systemd host)  
**Project Type**: CLI / reconciler  
**Performance Goals**: Planner remains O(workloads²) worst-case — unchanged  
**Constraints**: No new I/O in `plan()`; purely functional transformation  
**Scale/Scope**: Handful of workloads per host; no scale concern

## Constitution Check

- ✅ **Functional core vs. imperative shell**: all changes are in pure `plan()` and
  `build_apply_output`; no new I/O added.
- ✅ **Declarative state model**: the fix derives restart intent from the
  dependency graph encoded in desired state, consistent with the existing model.
- ✅ **Simplicity**: one new helper function + a post-loop pass in `plan()`; no
  new abstractions.
- ✅ **Explicit effects/failures**: `RestartUnit` is now an explicit action in
  the executable plan; failures propagate via `failed_actions` in the
  convergence record.
- ✅ **Idempotence**: re-applying identical desired state produces the same no-op
  diff → no `RestartUnit` scheduled → unchanged.
- ✅ **Observability**: `format_plan_report` already prints all actions including
  new `RestartUnit` entries; no further instrumentation needed.
- ✅ **Safe defaults**: scheduling a restart on config change is strictly safer
  than silently leaving stale state.
- ✅ **Compatibility**: no change to public CLI flags, state schema, or
  deterministic plan classification. The only externally observable change is
  that services now actually restart on config change — which is the correct
  behaviour operators already expected.
- ✅ **Release version policy**: patch bump; behaviour fix with no schema or
  contract change.
- ✅ **Release intent artifact**: `changes/014-config-restart-fidelity.md` with
  `release_intent: patch`.
- ✅ **Changelog**: `CHANGELOG.md` `Fixed` entry required before merge.
- ✅ **Test contract**: `cargo test` and `cargo clippy --all-targets -- -D warnings`;
  regression tests added in `tests/integration/`.

## Project Structure

### Documentation (this feature)

```text
specs/014-config-restart-fidelity/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
└── checklists/
    └── requirements.md
```

### Source Code (affected paths)

```text
src/
├── core/
│   ├── planner.rs        # plan() — add dependent-restart pass (P1)
│   └── evaluate.rs       # dependency_refs_for_workload_state — read-only
└── cli/
    └── report.rs         # build_apply_output — P2 validation

tests/
└── integration/
    └── test_plan.rs      # regression tests (P3)

changes/
└── 014-config-restart-fidelity.md   # release fragment
```

**Structure Decision**: Single Rust project; modifying two existing source files
plus existing integration test file; no new modules.

---

## Phase 0: Research

### Decision: Dependent lookup via `dependency_refs_for_workload_state`

**Decision**: Use `dependency_refs_for_workload_state(desired, workload)` from
`src/core/evaluate.rs` to find containers that depend on a changed config file.

**Rationale**: This function already parses quadlet `EnvironmentFile=` directives
against `desired.managed_config_paths` and produces the same ref strings used
as dependency edges in the semantic graph. The config file workload's
`systemd_unit_name` IS its target path (e.g., `/etc/github-actions-runner/start-runner.sh`),
which is exactly what `dependency_refs_for_workload_state` emits for `EnvironmentFile`
references. No new parsing needed.

**Alternative considered**: Building a `SemanticDependencyGraph` inside `plan()`
and using `dependent_refs`. Rejected because it requires building a `NormalizedSnapshot`
from desired state, which is heavier and adds an intermediate representation. The
direct workload scan via `dependency_refs_for_workload_state` is sufficient and
cheaper.

### Decision: Observed state as Add-case discriminator

**Decision**: For `DiffKind::Add` config file diffs, emit `RestartUnit` only for
dependent containers whose `systemd_unit_name` appears in `observed.workloads`.

**Rationale**: An observed workload is an already-managed container. If it's
managed but was started before the config file existed, it needs a restart to
pick up the new file. A container absent from observed state is being freshly
started; its own `StartUnit` action is sufficient.

**Alternative considered**: Using `observed.units` (systemd active-state). Rejected
because `observed.units` requires a live systemd query and may not be populated in
all contexts. `observed.workloads` is always populated and is the right proxy.

### Decision: P2 resolution via convergence-verification path

**Decision**: P2 (report sourcing accuracy) is resolved as a consequence of P1
combined with the existing post-apply verification path. After P1, a dependent
container has a real `RestartUnit` in the executable plan; if the restart fails,
`verify_state` detects the inactive unit and populates `failed_actions`, causing
`convergence_failed_for_entry` to return true and the report to show `failed`
instead of `restarted`.

**Rationale**: `build_apply_output` already checks `convergence_failed_for_entry`
to override the deterministic-plan terminal state with `Failed`. No additional
report-sourcing logic is required; the gap was that the executable plan had no
`RestartUnit` to execute or fail on.

**Alternative considered**: Adding a `completed_actions` membership check in
`build_apply_output` to downgrade `Restarted` to `Unchanged` when the restart
wasn't verified. Rejected because `completed_actions` is populated only from
verification results, not from `apply_plan` execution events. In non-verification
applies, `completed_actions` is empty and the check would incorrectly suppress
all `Restarted` terminal states.

---

## Phase 1: Design & Contracts

### Data Model Changes

No new types. Existing types involved:

- `PlanAction { action_type: PlanActionType::RestartUnit, target: String }` — no
  change; this variant already exists and is used for socket drop-in and container
  restarts.
- `DesiredState.workloads: Vec<Workload>` — read-only; the new pass iterates this.
- `ObservedState.workloads: Vec<Workload>` — read-only; used as Add-case
  discriminator.
- `DiffItem { name, kind, desired, observed }` — read-only; the new pass filters
  for `ConfigFile` diffs.

### Implementation Contract: `plan()` dependent-restart pass

Location: `src/core/planner.rs`, after the main `for diff in &diffs` loop.

```
INPUT:
  diffs: Vec<DiffItem>         — already computed, ordered
  desired: &DesiredState       — source of workload dependency refs and managed config paths
  observed: &ObservedState     — source of Add-case discriminator

LOGIC (pseudo-code):
  let observed_unit_names: HashSet<String> = observed.workloads
    .iter().map(|w| w.systemd_unit_name.clone()).collect();

  let already_restarted: HashSet<String> = actions.iter()
    .filter(|a| a.action_type == PlanActionType::RestartUnit)
    .map(|a| a.target.clone()).collect();

  for diff in &diffs where diff.quadlet_type == ConfigFile
                      and diff.kind in [Add, Change, Remove]:
    for workload in &desired.workloads:
      let deps = dependency_refs_for_workload_state(desired, workload);
      if deps.contains(&diff.name):
        let should_restart = match diff.kind {
          Add    => observed_unit_names.contains(&workload.systemd_unit_name),
          _      => true,
        };
        if should_restart && !already_restarted.contains(&workload.systemd_unit_name):
          actions.push(action(PlanActionType::RestartUnit, &workload.systemd_unit_name));
          already_restarted.insert(workload.systemd_unit_name.clone());

OUTPUT:
  actions: Vec<PlanAction>     — extended with dependent RestartUnit entries
```

**Ordering**: Config file diffs produce `WriteQuadlet` first (order 0 in
`order_for_type`). Container diffs follow (order 5). The dependent-restart pass
appends `RestartUnit` entries after all per-diff actions. Since actions are
executed sequentially in `apply.rs`, the config file is written before the
container is restarted. No ordering change needed.

**Deduplication**: `already_restarted` set prevents double scheduling. It is
initialised from the already-accumulated `actions` slice, so containers that
already have a `RestartUnit` from their own diff are skipped.

### Implementation Contract: Convergence failure surface (P2)

No code change required. Verification: confirm in the regression tests that when
a container's `RestartUnit` is in the executable plan and the service fails to
come up, the convergence record marks it as a failed action (via `verify_state`).
This is a documentation/test validation, not a new code path.

### Regression Test Contract (P3)

Location: `tests/integration/test_plan.rs` (or a new `test_planner.rs` — use
existing file to keep config-related tests co-located with
`desired_snapshot_extracts_config_and_runtime_dependency_refs`).

Tests to add:

**T1** — `config_file_change_schedules_restart_for_dependent_container`

```
GIVEN:
  desired: DesiredState with:
    workloads: [
      ConfigFile { systemd_unit_name: "/etc/runner/env", quadlet_contents: "KEY=newval" }
      Container { systemd_unit_name: "runner.container",
                  quadlet_contents: "[Container]\nEnvironmentFile=/etc/runner/env\n" }
    ]
    managed_config_paths: ["/etc/runner/env"]
  observed: ObservedState with:
    workloads: [
      ConfigFile { systemd_unit_name: "/etc/runner/env", quadlet_contents: "KEY=oldval" }
      Container { systemd_unit_name: "runner.container", ... }  ← already present
    ]
WHEN: plan(desired, observed) is called
THEN: actions contains RestartUnit for "runner.container"
      actions does NOT contain two RestartUnit entries for "runner.container"
      WriteQuadlet for "/etc/runner/env" precedes RestartUnit for "runner.container"
```

**T2** — `config_file_change_no_restart_when_no_dependents`

```
GIVEN:
  desired: DesiredState with:
    workloads: [ ConfigFile { systemd_unit_name: "/etc/runner/env" } ]
    managed_config_paths: ["/etc/runner/env"]
  observed: same config file with different contents
WHEN: plan(desired, observed)
THEN: no RestartUnit actions
```

**T3** — `config_file_add_restarts_already_running_container`

```
GIVEN:
  desired: DesiredState with config file + container (EnvironmentFile dependency)
  observed: config file ABSENT, container PRESENT in observed.workloads
WHEN: plan(desired, observed)
THEN: RestartUnit for container is scheduled  (Add + pre-existing container)
```

**T4** — `config_file_add_no_restart_for_new_container`

```
GIVEN:
  desired: config file + container (EnvironmentFile dependency)
  observed: neither config file nor container present
WHEN: plan(desired, observed)
THEN: no RestartUnit for container  (StartUnit from container's own diff is sufficient)
```

**T5** — `config_file_remove_schedules_restart_for_dependent_container`

```
GIVEN:
  desired: container ONLY (no config file)
  observed: config file + container BOTH present
WHEN: plan(desired, observed)
THEN: RestartUnit for container is present (surfacing the missing-file failure)
```

**T6** — `config_file_change_no_duplicate_restart_when_container_also_changed`

```
GIVEN:
  desired + observed: both config file and container changed
WHEN: plan(desired, observed)
THEN: exactly one RestartUnit for the container
```

All tests call `core_ops::core::planner::plan` (the executable planner) directly,
not the deterministic planner. No systemd, no filesystem, no temp dirs.

### Quickstart

See `specs/014-config-restart-fidelity/quickstart.md`.

---

## Complexity Tracking

No constitution violations. No complexity exceptions required.
