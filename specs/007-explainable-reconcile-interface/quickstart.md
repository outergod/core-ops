# Quickstart: Explainable Reconciliation Interface

## Purpose

Validate the new authoritative reconciliation output contract for planning, apply progress, final results, and object explanation before tasks are decomposed or implementation is accepted.

## Prerequisites

- Rust toolchain compatible with the repository (`edition = 2021`)
- Working tree on branch `007-explainable-reconcile-interface`
- Existing deterministic reconciliation fixtures and integration tests available

## Workflow

### 1. Review the contract inputs

Read:

- [spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/007-explainable-reconcile-interface/spec.md)
- [data-model.md](/home/outergod/code/github.com/outergod/core-ops/specs/007-explainable-reconcile-interface/data-model.md)
- [reconciliation-output.md](/home/outergod/code/github.com/outergod/core-ops/specs/007-explainable-reconcile-interface/contracts/reconciliation-output.md)

Confirm the feature is replacing the current plan JSON contract in place and is not changing persisted provenance schemas unless a separate, explicit migration is introduced.

### 2. Identify implementation touchpoints

Review the current implementation surfaces:

- `src/core/types.rs`
- `src/core/planner.rs`
- `src/cli/report.rs`
- `src/cli/plan.rs`
- `src/cli/apply.rs`
- `src/io/state.rs`

Review the current contract and behavioral tests:

- `tests/integration/test_status_contract.rs`
- `tests/integration/test_deterministic_planning.rs`
- `tests/integration/test_apply_report.rs`
- `tests/integration/test_reconcile_apply.rs`

### 3. Implement the new view model

Expected implementation sequence:

1. Add new operator-facing machine-readable types for plan/apply/result/explain output.
2. Add transformation logic from deterministic planning, drift, graph, verification, and convergence inputs into those public types.
3. Replace the legacy plan JSON renderer with the new `PlanOutput` shape.
4. Extend apply/result/explain renderers so human-readable output derives from the same public model.
5. Keep persisted provenance/deterministic state schemas unchanged unless deliberately versioned as separate work.

### 4. Update contract tests and documentation

Replace legacy expectations by updating:

- the plan JSON assertions in `tests/integration/test_status_contract.rs`
- any integration tests that match old plan field names or old classification semantics
- the prior contract document superseded by this feature

### 5. Verify behavior

Run:

```bash
cargo test
```

Exercise the human-facing surfaces with a retained prior revision:

```bash
core-ops apply --repo ./repo --rev demo-uat-v1
core-ops plan --repo ./repo --rev demo-uat-v2
core-ops apply --repo ./repo --rev demo-uat-v2
core-ops explain container/frontend.container
```

Focus review on:

- deterministic ordering
- stable object identity across views
- parity between human-readable and machine-readable output
- coverage of changed, unchanged, blocked, skipped, and failed cases
- convergence classification and revision context
- immutable short revision primary with meaningful requested refs rendered secondarily in human output
- prior requested-ref continuity after a revision has been retained by the current build

## Acceptance checklist

- New machine-readable plan output matches the contract in `contracts/reconciliation-output.md`
- Human-readable plan/apply/result rendering is derived from the same public model
- Legacy plan JSON contract is no longer normative in tests or docs
- Persisted state compatibility remains unchanged unless explicitly versioned
- Contract tests and scenario tests pass under `cargo test`
- Human headers and explain context render `<short-hash> (<requested-ref>)` when the ref is meaningful and preserve prior requested-ref context after successful retention
