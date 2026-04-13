# Data Model: Config Change Restart Fidelity

No new types are introduced. This document records which existing types are
touched and how.

## Read-only types (no structural change)

| Type | Location | Role in this fix |
|------|----------|-----------------|
| `DiffItem` | `src/core/types.rs` | Iterated in the new dependent-restart pass; filtered by `QuadletType::ConfigFile` |
| `DesiredState` | `src/core/types.rs` | `.workloads` and `.managed_config_paths` read to resolve dependencies |
| `ObservedState` | `src/core/types.rs` | `.workloads` used as Add-case discriminator |
| `Workload` | `src/core/types.rs` | `.systemd_unit_name`, `.quadlet_contents`, `.quadlet_type` read |

## Modified type usage

| Type | Location | Change |
|------|----------|--------|
| `ReconciliationPlan.actions: Vec<PlanAction>` | `src/core/types.rs` | New `RestartUnit` entries appended by the dependent-restart pass |

## No-change types

`DeterministicReconciliationPlan`, `PlanEntry`, `PlanEntryAction`,
`DeterministicActionClass` — these are NOT modified. The deterministic plan
path already classifies dependent containers as `Restart` and is the correct
source of truth for the plan-output view. Only the executable plan gains new
`RestartUnit` actions.

## Key relationship: config path as shared key

```
Workload (ConfigFile)
  .systemd_unit_name = "/etc/runner/env"   ← the target path
                          ↕  (same string)
Workload (Container)
  .quadlet_contents  = "[Container]\nEnvironmentFile=/etc/runner/env\n"
                              ↓  parsed by dependency_refs_for_workload_state
  dependency_refs    = ["/etc/runner/env"]
```

The `diff.name` for a `ConfigFile` diff equals `workload.systemd_unit_name`
which equals the path stored in `dependency_refs`. This identity is the
foundation of the lookup in the dependent-restart pass.
