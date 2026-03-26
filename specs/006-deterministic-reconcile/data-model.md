# Data Model: Deterministic Reconciliation

## DesiredStateRevision

- Purpose: Fully resolved target state for a specific reconciliation revision and managed scope.
- Fields:
  - `revision_id`: canonical desired-state revision identifier
  - `scope_id`: managed scope identifier for the host or selected subset
  - `objects`: ordered collection of normalized `ManagedObject` records representing desired state
  - `normalization_version`: normalization rule version used to produce this snapshot
  - `source_metadata`: repository/ref metadata needed for provenance and explanation
- Relationships:
  - Compared against zero or one `AppliedStateSnapshot`
  - Compared against one `ObservedStateSnapshot`
  - Produces one `ReconciliationPlan`
- Validation rules:
  - `revision_id` must be present and stable for the selected repository state
  - Object identities must be unique within the managed scope
  - Object ordering must be canonical and deterministic

## AppliedStateSnapshot

- Purpose: Last successfully applied normalized state retained for three-way planning and rollback.
- Fields:
  - `revision_id`: successful reconciliation revision identifier
  - `scope_id`: managed scope identifier
  - `objects`: ordered collection of normalized `ManagedObject` records representing the known-good applied state
  - `dependency_fingerprint`: stable summary or reconstructable dependency metadata for the applied graph
  - `applied_at`: completion timestamp for the successful apply boundary
  - `retention_class`: indicates whether the snapshot remains rollback-eligible within the configured history window
- Relationships:
  - Used as the historical baseline for `ReconciliationPlan`
  - Referenced by `RollbackTarget`
- Validation rules:
  - Must only be written after post-apply verification confirms convergence
  - Must not exist in a partially applied state for the same `revision_id`
  - Scope and normalization version must be compatible with current planning

## ObservedStateSnapshot

- Purpose: Canonical normalized view of actual host state at planning or verification time.
- Fields:
  - `observed_at`: observation timestamp
  - `scope_id`: managed scope identifier
  - `objects`: ordered collection of normalized `ManagedObject` records representing actual state
  - `runtime_signals`: relevant runtime status needed for verification and drift classification
- Relationships:
  - Compared against `DesiredStateRevision` and optional `AppliedStateSnapshot`
  - Used to derive `DriftRecord` entries and convergence outcomes
- Validation rules:
  - Object identities must be unique within the observed scope
  - Observation must include enough runtime data to classify blocked, converged, or divergent outcomes for managed objects

## ManagedObject

- Purpose: Canonical planning unit for a CoreOps-managed resource instance.
- Fields:
  - `object_id`: stable identity within the managed scope
  - `object_kind`: generated unit, Quadlet resource, mount, automount, rendered artifact, or another participating managed kind
  - `desired_payload`: normalized semantic content when present in desired state
  - `applied_payload`: normalized semantic content when present in last applied state
  - `actual_payload`: normalized semantic content when present in observed state
  - `dependency_refs`: explicit and implicit dependency references
  - `material_fields`: resource-kind-specific field set that participates in semantic comparison
- Relationships:
  - Represented as a node in `DependencyGraph`
  - Evaluated into one `PlannedAction`
  - May produce zero or more `DriftRecord` entries
- Validation rules:
  - `object_id` must be stable across desired, applied, and actual representations for the same resource
  - `object_kind` must map to documented normalization rules

## DependencyGraph

- Purpose: Explicit semantic graph for deterministic ordering, rollback, impact analysis, and cycle detection.
- Fields:
  - `nodes`: collection of `DependencyNode` records keyed by `object_id`
  - `edges`: collection of `DependencyEdge` records
  - `graph_fingerprint`: stable summary for reproducibility checks
- Relationships:
  - Built from `DesiredStateRevision`, `AppliedStateSnapshot`, and managed resource semantics
  - Used by `ReconciliationPlan`
- Validation rules:
  - Node identities must be unique
  - Edge direction must be semantically documented
  - Cycles must be rejected unless reduced by a documented rule

## DependencyNode

- Fields:
  - `object_id`
  - `object_kind`
  - `ordering_key`: deterministic tie-break key for topological ordering

## DependencyEdge

- Fields:
  - `from_object_id`
  - `to_object_id`
  - `edge_kind`: explicit or implicit
  - `reason`: concise causal explanation

## ReconciliationPlan

- Purpose: Ordered outcome of three-way planning for a reconciliation attempt.
- Fields:
  - `plan_id`: stable identifier for the plan instance
  - `desired_revision_id`
  - `baseline_revision_id`: optional last-applied revision identifier
  - `scope_id`
  - `actions`: ordered list of `PlannedAction`
  - `drift_records`: ordered list of `DriftRecord`
  - `blocked`: boolean summary indicator
  - `summary`: concise aggregate counts and rationale
- Relationships:
  - Consumes the three normalized snapshots and dependency graph
  - Drives apply execution and later verification
- Validation rules:
  - Action order must be deterministic for identical inputs
  - Every planned action must reference an existing `ManagedObject` identity

## PlannedAction

- Purpose: Single object-level reconciliation decision.
- Fields:
  - `action_id`: stable identifier within the plan
  - `object_id`
  - `classification`: create, update, delete, replace, no-op, or blocked
  - `dependency_context`: prerequisite and dependent references relevant to the action
  - `reason`: concise explanation of why this action exists
  - `semantic_diff`: material change summary
  - `expected_disruption`: none, bounded, or disruptive
- Relationships:
  - Belongs to one `ReconciliationPlan`
  - May contribute to one `ConvergenceRecord`
- Validation rules:
  - `classification` must be one of the approved action kinds
  - Blocked actions must include an explicit cause
  - Replace actions must identify removal and recreation implications

## DriftRecord

- Purpose: Structured record of a meaningful divergence seen during planning or verification.
- Fields:
  - `object_id`
  - `category`: expected change, external drift, stale residue, or runtime variance
  - `comparison_basis`: desired-vs-applied, actual-vs-applied, actual-vs-desired, or combined reasoning
  - `auto_action`: whether CoreOps intends automatic correction
  - `attention_required`: whether operator attention is required
  - `details`: concise semantic difference summary
- Validation rules:
  - `category` must be one of the documented drift classes
  - Runtime variance must be explicitly documented per resource kind if tolerated

## RollbackTarget

- Purpose: Selected previous successful revision to restore.
- Fields:
  - `target_revision_id`
  - `scope_id`
  - `eligibility_status`: eligible, missing_snapshot, incompatible_scope, or expired
  - `eligibility_reason`: concise explanation
- Relationships:
  - Resolved against retained `AppliedStateSnapshot` history
  - Planned through `ReconciliationPlan`
- Validation rules:
  - Only retained successful revisions within the rollback window may be eligible

## ConvergenceRecord

- Purpose: Structured post-apply outcome for success, partial progress, or non-convergence.
- Fields:
  - `attempt_id`
  - `desired_revision_id`
  - `scope_id`
  - `status`: success, partial, blocked, repeated_failure, oscillation, or failed
  - `attempt_count`: bounded retry count for the same object set and failure pattern
  - `affected_objects`: ordered list of object identities involved in the outcome
  - `completed_actions`: ordered list of completed action identifiers
  - `failed_actions`: ordered list of failed or blocked action identifiers
  - `remaining_drift`: ordered list of unresolved `DriftRecord` identities or summaries
  - `can_continue`: whether a later reconcile can continue from the current state
- Validation rules:
  - Success requires post-apply verification convergence
  - `attempt_count` must never exceed the configured retry budget without surfacing intervention-required status

## State Transitions

### Reconciliation attempt lifecycle

- `planned` -> `executing` -> `verifying` -> `success`
- `planned` -> `executing` -> `verifying` -> `partial`
- `planned` -> `executing` -> `verifying` -> `blocked`
- `planned` -> `executing` -> `verifying` -> `repeated_failure`
- `planned` -> `executing` -> `verifying` -> `oscillation`
- `planned` -> `executing` -> `verifying` -> `failed`

### Rollback eligibility lifecycle

- `eligible` -> `expired` when the retained snapshot leaves the rollback window
- `eligible` -> `incompatible_scope` when the retained snapshot no longer safely covers the managed scope
- `eligible` -> `missing_snapshot` when required retained state is removed or unavailable
