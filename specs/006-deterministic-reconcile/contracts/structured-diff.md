# Contract: Structured Diff Output

## Status

This legacy deterministic structured-diff contract is superseded for
operator-facing plan output.

## Current Normative Contract

The authoritative machine-readable reconciliation output contract now lives in:

- `specs/007-explainable-reconcile-interface/contracts/reconciliation-output.md`

That contract defines:

- `view_kind`
- `revision_context`
- `ManagedObjectRef`
- `Cause`
- `DependencyEdge`
- `SemanticDiff`
- `PlanOutput.entries`

## Migration Note

Any tests, renderers, or external consumers that still rely on legacy top-level
fields such as `scope_id`, `actions`, `drift`, or `graph` should be updated to
the new contract immediately. No parallel versioned legacy plan schema remains
normative after the explainable reconciliation interface release.
