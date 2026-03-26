# Contract: Structured Diff Output

## Purpose

Define the minimum machine-readable contract for reconciliation plan output and reconciliation result output introduced by deterministic reconciliation.

## Scope

Applies to dry-run plan output, apply-time reported plan output, and persisted or streamed reconciliation result summaries that expose action-level reasoning.

## Required top-level fields

- `schema_version`: integer contract version
- `scope_id`: managed scope identifier
- `desired_revision_id`: selected desired revision
- `baseline_revision_id`: optional last-applied revision used for three-way reasoning
- `plan_id`: stable identifier for this plan or result set
- `blocked`: boolean aggregate indicating whether any action is blocked
- `actions`: ordered array of action records
- `drift`: ordered array of drift records
- `summary`: aggregate counts and concise textual summary

## Action record contract

Each `actions[]` entry MUST include:

- `object_id`
- `object_kind`
- `classification`: `create`, `update`, `delete`, `replace`, `no-op`, or `blocked`
- `order_index`: deterministic execution order index
- `dependency_context`
- `reason`
- `semantic_diff`
- `expected_disruption`

### Dependency context

`dependency_context` MUST carry enough information to explain why the action is ordered where it is. At minimum it MUST identify:

- prerequisites or predecessors
- affected dependents when relevant
- whether the dependency edge is explicit or implicit

### Semantic diff

`semantic_diff` MUST include only materially relevant differences after normalization. Formatting-only differences MUST NOT appear as actionable change.

## Drift record contract

Each `drift[]` entry MUST include:

- `object_id`
- `category`: `expected_change`, `external_drift`, `stale_residue`, or `runtime_variance`
- `comparison_basis`
- `auto_action`
- `attention_required`
- `details`

## Result contract extensions

When the contract is emitted for a reconciliation result, it MUST additionally include:

- `result_status`: `success`, `partial`, `blocked`, `repeated_failure`, `oscillation`, or `failed`
- `attempt_count`
- `completed_actions`
- `failed_actions`
- `remaining_drift`
- `can_continue`

## Determinism rules

- Repeated planning with identical normalized inputs MUST produce materially identical `actions`, `drift`, and `summary` content.
- Array ordering MUST be stable and deterministic.
- Human-readable rendering MUST be derived from this machine-readable representation and MUST NOT introduce facts absent from it.
