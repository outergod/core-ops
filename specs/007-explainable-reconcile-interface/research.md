# Research: Explainable Reconciliation Interface

## Decision: Replace the current deterministic plan JSON contract in place after enriching the internal view model

### Rationale

- The current deterministic plan already contains the core planning facts needed for migration: ordered actions, drift records, dependency graph, desired revision, and baseline revision.
- The main gap is contract shape and typed explanation data, not absence of planning semantics.
- An in-place replacement is feasible if the planner/view layer first produces structured identities, causes, dependency relations, semantic diffs, and full-scope entries, then swaps the renderer and contract tests in the same release scope.
- This honors the spec clarification that no parallel versioned plan schema should remain normative after release.

### Alternatives considered

- Keep the current JSON as canonical and defer the target schema.
  Rejected because it would fail the feature's primary contract.
- Add a parallel `v2` plan schema.
  Rejected because the active spec explicitly chose immediate in-place replacement.
- Swap only the renderer and infer the new schema from existing strings.
  Rejected because it would be lossy and brittle for identity, cause typing, diff typing, and unchanged-object coverage.

## Decision: Treat the schema replacement as an explicit compatibility event, but do not couple it to persisted-state migration

### Rationale

- The executable contract today is enforced by integration tests around plan and convergence JSON, especially `tests/integration/test_status_contract.rs`.
- The old deterministic contract is still documented in `specs/006-deterministic-reconcile/contracts/structured-diff.md`; both tests and docs must change together.
- Persisted provenance and deterministic state already have explicit schema-version handling and fixtures. Changing them in the same feature would increase migration risk without being necessary for the interface contract itself.
- The constitution permits unavoidable breakage only when it is explicit, documented, versioned, and justified. This feature therefore requires coordinated contract-test updates, documentation replacement, and release-version-policy review.

### Alternatives considered

- Migrate persisted provenance and deterministic state schemas at the same time.
  Rejected because the feature is about output contracts, not on-disk state evolution.
- Update renderers now and leave tests/docs for later.
  Rejected because it would leave the repository with contradictory normative contracts.
- Preserve both old and new machine-readable contracts temporarily.
  Rejected because it prolongs compatibility burden and contradicts the chosen clarification.

## Decision: Add dedicated operator-facing schema types instead of reusing deterministic planner structs as the public contract

### Rationale

- The target spec defines richer public concepts than the current deterministic core: `ManagedObjectRef`, `RevisionContext`, `Cause`, `DependencyEdge`, `SemanticDiff`, and full view shapes for plan, apply, result, and explain output.
- Existing deterministic types remain useful internal inputs: `ManagedObjectKind`, `SemanticDependencyGraph`, `StructuredDriftRecord`, `DeterministicConvergenceRecord`, and persisted rollback/provenance records.
- A separate public model preserves the constitutional split between core planning data and operator-facing render data while avoiding leaky planner internals.

### Alternatives considered

- Reuse `DeterministicReconciliationPlan` directly as the public schema.
  Rejected because it cannot express structured object identity, typed causes, or the full plan/apply/result/explain view model without semantic overload.
- Replace internal deterministic types wholesale with the public schema.
  Rejected because planner/provenance internals and public reporting have different responsibilities and stability needs.
- Use raw JSON assembly without dedicated types.
  Rejected because the feature's goal is stable, contract-tested, machine-readable semantics.
