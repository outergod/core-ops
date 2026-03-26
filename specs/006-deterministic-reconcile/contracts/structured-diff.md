# Contract: Structured Diff Output

## Purpose

Define the minimum machine-readable contract for deterministic reconciliation
plan output and apply/verification result output in this iteration.

## Scope

Applies to dry-run plan output, apply-time deterministic plan reporting, and
structured convergence reporting emitted after apply.

## Plan output contract

The machine-readable deterministic plan output MUST include these top-level
fields:

- `scope_id`: managed scope identifier
- `desired_revision_id`: selected desired revision
- `baseline_revision_id`: optional last-applied revision used for three-way
  reasoning
- `actions`: ordered array of action records
- `drift`: ordered array of drift records
- `graph`: semantic dependency graph used for ordering and explanation

### Action record contract

Each `actions[]` entry MUST include:

- `object_id`
- `classification`: `create`, `update`, `delete`, `replace`, `no_op`, or
  `blocked`
- `reason`
- `dependency_context`: ordered list of prerequisite or dependent object
  references relevant to the action
- `semantic_diff`: material field-level differences after normalization

### Drift record contract

Each `drift[]` entry MUST include:

- `object_id`
- `category`: `expected_change`, `external_drift`, `stale_residue`, or
  `runtime_variance`
- `comparison_basis`
- `auto_action`
- `attention_required`
- `details`

### Graph contract

`graph` MUST include:

- `nodes[]` with `object_id`, `object_kind`, and `ordering_key`
- `edges[]` with `from_object_id`, `to_object_id`, `edge_kind`, and `reason`

Supported `object_kind` values in this iteration are:

- `generated_unit`
- `quadlet_resource`
- `mount`
- `automount`
- `rendered_artifact`

Supported `edge_kind` values are:

- `explicit`
- `implicit`

## Apply and convergence result contract

The structured apply result emitted after deterministic reconciliation MUST
include:

- `run_id`
- `status`
- `summary`
- `verification_results[]`
- optional `convergence`

When `convergence` is present it MUST include:

- `desired_revision_id`
- `scope_id`
- `status`
- `attempt_count`
- `affected_objects`
- `completed_actions`
- `failed_actions`
- `can_continue`

## Human-readable rendering rules

- Human-readable plan output MUST be derivable from the machine-readable plan
  representation.
- Human-readable rendering MUST preserve ordered actions, drift categories, and
  dependency context without introducing facts absent from the structured data.
- Rollback output may prepend rollback-target context, but the embedded plan
  portion MUST follow the same deterministic plan semantics.

## Determinism rules

- Repeated planning with identical normalized inputs MUST produce materially
  identical `actions`, `drift`, and `graph` content.
- Array ordering MUST be stable and deterministic.
- Formatting-only differences MUST NOT appear as actionable semantic diffs.
