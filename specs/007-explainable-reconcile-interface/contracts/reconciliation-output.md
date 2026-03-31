# Contract: Reconciliation Output

## Purpose

Define the authoritative machine-readable contract for CoreOps reconciliation planning, apply progress, final result reporting, and single-object explanation for feature `007-explainable-reconcile-interface`.

## Scope

This contract supersedes the legacy deterministic structured diff contract for operator-facing plan JSON. It applies to:

- plan output
- apply progress output
- final result output
- targeted explain output

Persisted provenance and deterministic state files are out of scope unless explicitly revised by a separate schema change.

## Compatibility Rules

- This feature replaces the currently implemented plan JSON contract in place.
- No parallel versioned plan schema remains normative after release.
- Documented field names, enum values, and ordering semantics are compatibility-sensitive.
- New optional fields may be added in compatible releases.
- Existing documented fields and meanings must remain stable within a compatible release line.
- Arrays with semantic ordering must preserve deterministic order consistent with reconciliation planning and presentation semantics.
- Human-readable output must be a rendering of this machine-readable contract and must not introduce semantics absent from it.

## Shared Entities

### ManagedObjectRef

Required fields:

- `resource_type`
- `name`
- `display_id`

### RevisionContext

Required fields:

- `target_revision`

Optional fields:

- `requested_repository`
- `requested_ref`
- `last_applied_revision`
- `last_applied_requested_repository`
- `last_applied_requested_ref`
- `change_revision`

### Cause

Required fields:

- `kind`
- `summary`

Optional fields:

- `source_object`
- `details`

Allowed `kind` values:

- `desired_change`
- `drift`
- `dependency_change`
- `dependency_failure`
- `blocked_prerequisite`
- `runtime_variance`
- `recovery_required`
- `replacement_required`
- `restart_required`
- `no_change`

### DependencyEdge

Required fields:

- `relation`
- `object`

Allowed `relation` values:

- `prerequisite`
- `dependent`
- `blocker`

### SemanticDiff

Required fields:

- `kind`
- `summary`

Optional fields:

- `unified_diff`
- `details`

Allowed `kind` values:

- `line_based`
- `semantic_only`
- `replacement`
- `deletion`
- `creation`

## Plan Output Contract

Top-level fields:

- `view_kind` with value `plan`
- `revision_context`
- `summary`
- `entries`

### Plan summary

Required fields:

- `changed_count`
- `unchanged_count`
- `blocked_count`
- `skipped_count`

Optional fields:

- `total_count`

### Plan entry

Required fields:

- `object`
- `action`
- `causes`
- `dependencies`
- `order_index`

Optional fields:

- `diff`
- `unchanged`
- `notes`

Allowed `action` values:

- `create`
- `update`
- `replace`
- `delete`
- `recover`
- `restart`
- `no_op`
- `blocked`
- `skipped`

Contract rules:

- Entries must cover the full reconciliation scope, including changed and unchanged objects.
- Changed objects must appear before unchanged objects in the default plan rendering, but the machine-readable `entries` array remains the authoritative ordered list.
- Non-no-op entries must contain at least one cause.
- Dependency edges in plan entries represent the prerequisite-oriented default explanation projection.

## Apply Output Contract

Top-level fields:

- `view_kind` with value `apply`
- `revision_context`
- `phases`
- `events`
- `summary` when the run reaches terminal completion

### Phase event

Required fields:

- `phase`
- `state`
- `sequence`

Allowed `phase` values:

- `resolution`
- `graph_construction`
- `planning`
- `execution`
- `convergence_check`
- `final_summary`

Allowed `state` values:

- `started`
- `completed`
- `failed`

### Execution event

Required fields:

- `object`
- `event_kind`
- `state`
- `sequence`

Optional fields:

- `action`
- `cause`
- `phase`
- `impacted_objects`

Allowed `event_kind` values:

- `object_progress`
- `object_terminal`
- `object_blocked`
- `object_skipped`

Allowed `state` values:

- `pending`
- `running`
- `created`
- `updated`
- `deleted`
- `recovered`
- `restarted`
- `failed`
- `blocked`
- `unchanged`
- `skipped`

Contract rules:

- Sequence ordering is deterministic narration order, not necessarily internal execution order.
- Repeated runs with materially identical inputs and outcomes must yield materially identical phase and event ordering.

## Result Output Contract

Top-level fields:

- `view_kind` with value `result`
- `revision_context`
- `outcome`
- `summary`
- `entries`

Allowed `outcome` values:

- `converged`
- `converged_with_tolerated_variance`
- `partially_applied`
- `failed`
- `non_converging`

### Result summary

Required fields:

- `changed_count`
- `failed_count`
- `blocked_count`
- `skipped_count`
- `unchanged_count`

Optional fields:

- `message`

### Result entry

Required fields:

- `object`
- `final_state`

Optional fields:

- `action`
- `causes`
- `dependencies`
- `diff`
- `notes`

Allowed `final_state` values:

- `succeeded`
- `failed`
- `blocked`
- `skipped`
- `no_op`

## Explain Output Contract

Top-level fields:

- `view_kind` with value `explain`
- `revision_context`
- `object`
- `action_or_outcome`
- `causes`
- `dependencies`

Optional fields:

- `diff`
- `metadata`
- `x_coreops`
- `summary`

## Determinism and Rendering Rules

- Machine-readable output is the authoritative representation of reconciliation state.
- Human-readable output must preserve object identity, action meaning, dependency context, explanation semantics, and convergence classification from the machine-readable model.
- Formatting-only changes must not appear as material semantic differences.
- Stable object identity must be preserved across plan, apply, result, and explain views for the same managed object.

## Replacement Impact

Implementation for this feature must update:

- the JSON renderers in the CLI reporting path
- contract tests that currently assert the legacy deterministic plan shape
- integration tests that match legacy field names or output semantics
- any prior contract documentation that would otherwise remain normative
