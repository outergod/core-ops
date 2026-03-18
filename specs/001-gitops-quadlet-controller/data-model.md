# Data Model: GitOps Quadlet Controller

**Date**: 2026-03-18

## Entities

### DesiredState

- **Fields**: repository_ref, revision_id, workloads[], invariants[], boundaries
- **Validation**: workloads reference only supported Quadlet types; boundaries
  explicitly declared; invariants present before planning.

### Workload

- **Fields**: name, quadlet_type, quadlet_contents, systemd_unit_name,
  enabled_state, restart_policy
- **Validation**: quadlet_type supported; unit name deterministic from content;
  no unsupported directives for Fedora CoreOS.

### ObservedState

- **Fields**: observed_revision_id, units[], workloads[], last_reconcile_id,
  host_info
- **Validation**: units map to generated systemd units; observed workloads align
  with Quadlet generator output.

### ReconciliationPlan

- **Fields**: plan_id, desired_revision_id, observed_revision_id, actions[],
  safety_checks[], expected_outcomes[]
- **Validation**: actions are within supported boundaries; safety checks include
  invariant verification and boundary enforcement.

### PlanAction

- **Fields**: action_type (write_quadlet, remove_quadlet, enable_unit, disable_unit,
  reload_systemd, start_unit, stop_unit), target, preconditions[], postconditions[]
- **Validation**: action_type is allowed; preconditions reference observed state.

### ReconcileRun

- **Fields**: run_id, started_at, finished_at, mode (plan/apply), status
  (success/failure), failure_class, summary
- **Validation**: status aligns with verification results; failure_class set when
  status is failure.

### AuditRecord

- **Fields**: record_id, run_id, diffs, plan_summary, actions_applied,
  verification_results, operator_messages
- **Validation**: records are immutable per run; includes enough detail to
  explain reasoning.

## Relationships

- DesiredState -> Workload (1-to-many)
- ObservedState -> Workload (1-to-many)
- ReconciliationPlan -> PlanAction (1-to-many)
- ReconcileRun -> ReconciliationPlan (1-to-1)
- ReconcileRun -> AuditRecord (1-to-1)

## State Transitions

- Plan: DesiredState + ObservedState -> ReconciliationPlan
- Apply: ReconciliationPlan -> ObservedState update -> Verify
- Verify: ObservedState + DesiredState -> ReconcileRun status
