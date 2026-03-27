# Data Model: Explainable Reconciliation Interface

## Overview

This feature adds an operator-facing machine-readable reconciliation model on top of CoreOps' existing deterministic planner, convergence, and provenance data. Internal deterministic types remain authoritative inputs for planning and verification; new public view types become the authoritative contract for machine-readable output and the source for human-readable rendering.

## Entities

### ManagedObjectRef

Represents a managed object consistently across plan, apply, result, and explain views.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `resource_type` | enum/string | Yes | Stable public kind identifier derived from the managed object kind |
| `name` | string | Yes | Stable object name within the resource type |
| `display_id` | string | Yes | Canonical user-facing identifier derived from `resource_type` and `name` |

**Validation rules**
- `display_id` must be deterministic for a given `resource_type` and `name`.
- The same managed object must map to the same `ManagedObjectRef` across plan, apply, result, and explain views.

### RevisionContext

Provides provenance for the reconciliation scope shown in a view.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `target_revision` | string | Yes | Desired revision being planned or applied |
| `last_applied_revision` | string | No | Most recent successful revision for the same scope |
| `change_revision` | string | No | Revision associated with the reported change set when distinguishable |

**Validation rules**
- `target_revision` must always be present in public view outputs.
- Optional fields must preserve the spec's absent-versus-null semantics.

### Cause

Explains why a managed object has an action or outcome.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | enum | Yes | `desired_change`, `drift`, `dependency_change`, `dependency_failure`, `blocked_prerequisite`, `runtime_variance`, `replacement_required`, `restart_required`, `no_change` |
| `summary` | string | Yes | Concise explanation suitable for direct rendering |
| `source_object` | `ManagedObjectRef` | No | Present when the cause is driven by another managed object |
| `details` | object/map | No | Optional structured cause metadata |

**Validation rules**
- Non-no-op plan entries must have at least one cause.
- Cause ordering must be deterministic.

### DependencyEdge

Represents a public explanation relationship between a selected object and another managed object.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `relation` | enum | Yes | `prerequisite`, `dependent`, `blocker` |
| `object` | `ManagedObjectRef` | Yes | Related managed object |

**Validation rules**
- Default plan/apply dependency rendering uses prerequisite-oriented edges.
- Blockers must not be represented as ordinary prerequisites.

### SemanticDiff

Represents the material change evidence for a changed object.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | enum | Yes | `line_based`, `semantic_only`, `replacement`, `deletion`, `creation` |
| `summary` | string | Yes | Primary deterministic explanation of the change |
| `unified_diff` | string | No | Present only where line-based evidence is meaningful |
| `details` | object/map | No | Structured metadata for richer diffs |

**Validation rules**
- Diff content is supporting evidence; it cannot replace action/explanation semantics.
- Formatting-only differences must never appear as material semantic changes.

### PlanEntry

Represents one managed object in deterministic plan order.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `object` | `ManagedObjectRef` | Yes | Stable object identity |
| `action` | enum | Yes | `create`, `update`, `replace`, `delete`, `restart`, `no_op`, `blocked`, `skipped` |
| `causes` | array of `Cause` | Yes | Empty only for `no_op` entries |
| `dependencies` | array of `DependencyEdge` | Yes | Default explanation projection |
| `order_index` | integer | Yes | Deterministic linearization index |
| `diff` | `SemanticDiff` | No | Present for materially changed objects |
| `unchanged` | boolean | No | Convenience field only if retained consistently |
| `notes` | array | No | Supplemental non-normative notes |

**Relationships**
- Many `PlanEntry` values belong to one `PlanOutput`.
- Each `PlanEntry` may reference multiple `Cause` and `DependencyEdge` records.

### PlanSummary

Represents top-level counts for the plan view.

| Field | Type | Required |
|-------|------|----------|
| `changed_count` | integer | Yes |
| `unchanged_count` | integer | Yes |
| `blocked_count` | integer | Yes |
| `skipped_count` | integer | Yes |
| `total_count` | integer | No |

### PlanOutput

Represents the authoritative machine-readable result of planning.

| Field | Type | Required |
|-------|------|----------|
| `view_kind` | literal `plan` | Yes |
| `revision_context` | `RevisionContext` | Yes |
| `summary` | `PlanSummary` | Yes |
| `entries` | array of `PlanEntry` | Yes |

### PhaseEvent

Represents a phase transition during apply.

| Field | Type | Required |
|-------|------|----------|
| `phase` | enum | Yes |
| `state` | enum | Yes |
| `sequence` | integer | Yes |

**State transitions**
- `started` -> `completed`
- `started` -> `failed`

### ExecutionEvent

Represents one object-specific apply event.

| Field | Type | Required |
|-------|------|----------|
| `object` | `ManagedObjectRef` | Yes |
| `event_kind` | enum | Yes |
| `state` | enum | Yes |
| `sequence` | integer | Yes |
| `action` | enum | No |
| `cause` | `Cause` | No |
| `phase` | enum | No |
| `impacted_objects` | array of `ManagedObjectRef` | No |

**State transitions**
- `pending` -> `running` -> `succeeded`
- `pending` -> `running` -> `failed`
- `pending` -> `blocked`
- `pending` -> `skipped`

### ApplyOutput

Represents streamed or collected execution progress for a reconciliation run.

| Field | Type | Required |
|-------|------|----------|
| `view_kind` | literal `apply` | Yes |
| `revision_context` | `RevisionContext` | Yes |
| `phases` | array of `PhaseEvent` | Yes |
| `events` | array of `ExecutionEvent` | Yes |
| `summary` | object/string | Conditionally required at terminal completion |

### ResultEntry

Represents the final object-level outcome in a completed reconciliation run.

| Field | Type | Required |
|-------|------|----------|
| `object` | `ManagedObjectRef` | Yes |
| `final_state` | enum | Yes |
| `action` | enum | No |
| `causes` | array of `Cause` | No |
| `dependencies` | array of `DependencyEdge` | No |
| `diff` | `SemanticDiff` | No |

### ResultSummary

Represents the final top-level outcome counts.

| Field | Type | Required |
|-------|------|----------|
| `changed_count` | integer | Yes |
| `failed_count` | integer | Yes |
| `blocked_count` | integer | Yes |
| `skipped_count` | integer | Yes |
| `unchanged_count` | integer | Yes |
| `message` | string | No |

### ResultOutput

Represents the final authoritative machine-readable result of a reconciliation run.

| Field | Type | Required |
|-------|------|----------|
| `view_kind` | literal `result` | Yes |
| `revision_context` | `RevisionContext` | Yes |
| `outcome` | enum | Yes |
| `summary` | `ResultSummary` | Yes |
| `entries` | array of `ResultEntry` | Yes |

### ExplainOutput

Represents targeted inspection of a single managed object.

| Field | Type | Required |
|-------|------|----------|
| `view_kind` | literal `explain` | Yes |
| `revision_context` | `RevisionContext` | Yes |
| `object` | `ManagedObjectRef` | Yes |
| `action_or_outcome` | string/enum | Yes |
| `causes` | array of `Cause` | Yes |
| `dependencies` | array of `DependencyEdge` | Yes |
| `diff` | `SemanticDiff` | No |
| `history` | object/array | No |

## Internal Inputs Reused

These existing types remain inputs to the new public model rather than becoming the public model directly:

- `ManagedObjectKind`
- `SemanticDependencyGraph`
- `StructuredDriftRecord`
- `DeterministicReconciliationPlan`
- `DeterministicConvergenceRecord`
- `DeterministicPersistedState`
- `RollbackTargetCandidate`

## Key Relationships

- `DeterministicReconciliationPlan` feeds `PlanOutput`.
- `DeterministicConvergenceRecord` plus verification/apply progress feed `ApplyOutput` and `ResultOutput`.
- `SemanticDependencyGraph` and normalized snapshots feed `DependencyEdge`, `ManagedObjectRef`, `Cause`, and `SemanticDiff`.
- `ResultOutput` and `PlanOutput` provide the source data for `ExplainOutput`.

## Migration Notes

- The current `{scope_id, desired_revision_id, baseline_revision_id, actions, drift, graph}` JSON is not the target public schema after this feature.
- The migration should keep persisted provenance and deterministic state schemas unchanged unless a separate, explicit schema change is needed.
- Contract tests must validate new public view shapes directly rather than via legacy field-name compatibility.
