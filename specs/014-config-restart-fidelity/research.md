# Research: Config Change Restart Fidelity

## Decision Log

### D1 — Dependent lookup strategy

**Decision**: Use `dependency_refs_for_workload_state(desired, workload)` from
`src/core/evaluate.rs`.

**Rationale**: For `QuadletType::ConfigFile`, `systemd_unit_name` equals the
target path (e.g., `/etc/github-actions-runner/start-runner.sh`). This is the
same string that `dependency_refs_for_workload_state` emits when it parses an
`EnvironmentFile=/etc/github-actions-runner/start-runner.sh` directive in a
container quadlet. No new parsing is required; the lookup is a direct string
membership test.

**Key code reference**: `src/io/repo.rs:591` — `workload_from_config_file` sets
`systemd_unit_name: file.target_path.clone()`.

**Alternatives considered**: Build a `SemanticDependencyGraph` inside `plan()`.
Rejected: requires constructing a `NormalizedSnapshot` from desired state, which
adds an intermediate representation and crosses the concern boundary of the
functional planner core. Direct workload scan is cheaper and equivalent.

---

### D2 — Add-case discriminator

**Decision**: Use `observed.workloads` membership to determine whether a
container is "already running" for the `DiffKind::Add` case.

**Rationale**: `observed.workloads` contains all workloads currently on-disk
under the quadlet dir. If a container is observed, it was previously managed and
likely active. `observed.units` (live systemd active-state) is not used because
it requires a live systemd query and may be unpopulated in test and non-host
contexts.

---

### D3 — P2 report sourcing

**Decision**: No new `build_apply_output` code needed. The existing
`convergence_failed_for_entry` check (which reads `failed_actions` from the
`DeterministicConvergenceRecord`) already surfaces restart failures. After P1
adds `RestartUnit` to the executable plan, a failing restart causes `verify_state`
to mark the unit as failed → it lands in `failed_actions` → report shows `failed`.

**Key code reference**: `src/cli/report.rs:1374` — `convergence_failed_for_entry`;
`src/core/verify.rs:61-70` — `completed_actions` / `failed_actions` population.

---

### D4 — Deduplication

**Decision**: Track already-scheduled `RestartUnit` targets in a `HashSet` built
from the accumulated actions slice before the dependent-restart pass begins.

**Rationale**: Prevents double scheduling when a container independently changed
(producing its own `RestartUnit`) AND depends on a changed config file. A single
pass through the existing `actions` vec before the config file loop is O(n) and
sufficient.

---

### D5 — Ordering

**Decision**: Append dependent `RestartUnit` entries after all per-diff actions.
No reordering needed.

**Rationale**: `order_for_type` already places `ConfigFile` diffs at position 0
and `Container` diffs at position 5. After the main loop, config file
`WriteQuadlet` actions precede any container actions. The new `RestartUnit`
entries are appended last. `apply.rs` executes actions sequentially, so
`WriteQuadlet` for the config file will always run before the dependent
`RestartUnit`. Verified by test T1 (ordering assertion).
