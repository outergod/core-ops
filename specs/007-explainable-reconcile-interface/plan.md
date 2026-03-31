# Implementation Plan: Explainable Reconciliation Interface

**Branch**: `007-explainable-reconcile-interface` | **Date**: 2026-03-27 | **Spec**: [spec.md](/home/outergod/code/github.com/outergod/core-ops/specs/007-explainable-reconcile-interface/spec.md)
**Input**: Feature specification from `/specs/007-explainable-reconcile-interface/spec.md`

## Summary

Replace CoreOps' current deterministic plan JSON contract in place with the new authoritative machine-readable reconciliation model defined in the feature spec, while preserving the functional-core/imperative-shell split. The implementation will introduce new operator-facing view types for plan, apply, result, and explain output, derive them from existing deterministic planner and convergence data, update human-readable renderers to be pure renderings of the new model, and replace existing contract tests and legacy contract documentation in the same feature scope.

## Technical Context

**Language/Version**: Rust 2021 (`core-ops` 0.5.0)  
**Primary Dependencies**: `clap`, `miette`, `thiserror`, `serde`, `serde_json`, `log`, `systemd-journal-logger`, `tempfile`  
**Storage**: Files on disk for persisted provenance and deterministic reconciliation state under the runtime state directory; machine-readable interface payloads are emitted transiently by CLI/report surfaces  
**Testing**: `cargo test` with unit and integration suites, including contract-style JSON assertions in `tests/integration/test_status_contract.rs`  
**Target Platform**: Single-node Linux hosts using systemd and Quadlet-managed resources  
**Project Type**: CLI reconciler with persisted state and machine-readable/reporting contracts  
**Performance Goals**: For a representative single-node reconciliation scope used in acceptance tests, machine-readable and human-readable plan/result rendering should complete within 1 second per invocation while preserving identical ordering on repeated runs with materially identical inputs  
**Constraints**: Replace the current plan JSON contract in place; keep persisted provenance and deterministic state schemas stable unless their documented meaning changes; preserve machine/human semantic parity; maintain deterministic ordering despite future internal concurrency  
**Scale/Scope**: Single-node reconciliation scope across current CoreOps-managed resources, with full-scope plan/result coverage including changed and unchanged objects

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Functional core and imperative shell boundaries are explicit; side effects are isolated.**
  Pass. New machine-readable view types and renderers remain pure data/rendering logic in `src/core` and `src/cli/report.rs`; filesystem persistence and runtime execution stay in `src/io` and apply boundaries.
- **Desired/observed state, reconciliation plans, and outcomes are represented as data.**
  Pass. This feature extends that pattern by adding explicit operator-facing plan/apply/result/explain view data structures.
- **Abstractions are minimal and justified; complexity tracking added if needed.**
  Pass. New public contract types are justified because the target schema is richer than the current planner internals and should not be inferred ad hoc in renderers.
- **Effects, assumptions, and failure modes are explicit in interfaces and returns.**
  Pass. The plan will add typed causes, dependency relations, terminal states, and convergence/result classifications rather than relying on opaque strings alone.
- **Idempotence and convergence strategy are defined, including retry behavior.**
  Pass. Existing deterministic reconciliation and bounded retry semantics remain authoritative; the feature changes how they are surfaced, not their core execution model.
- **Open standards and native interfaces are preferred; deviations justified.**
  Pass. The interface remains JSON plus human-readable CLI output on top of existing systemd/Quadlet integration.
- **Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs.**
  Pass. The feature's main purpose is to make those surfaces explicit and contract-tested.
- **Provenance and status surfaces identify reconciler revision, desired-state revision, and applied outcome in machine-readable form.**
  Pass. Revision context is part of the new schema and now distinguishes mutable desired-state selection context (`requested_repository`, `requested_ref`) from the resolved immutable reconciliation identity (`target_revision`); persisted provenance remains stable while reconciliation and rollback stay anchored to the immutable revision.
- **Safe defaults are documented; destructive actions require explicit intent.**
  Pass. The feature is observational/presentational and preserves explicit reporting of blocked, skipped, partial, and failed outcomes.
- **Compatibility impact is assessed; breaking changes are documented with migration.**
  Pass with explicit compatibility event. Replacing the plan JSON contract in place is a deliberate breaking change to externally observable machine-readable output and will require test/doc updates plus release-version-policy review.
- **Release version policy impact is assessed for any externally observable, schema, CLI, reconciliation, or compatibility change; the canonical controller version comes from `Cargo.toml`.**
  Pass with action required in implementation. This feature changes externally observable machine-readable schema and human-readable rendering semantics; release policy review and likely package version update are part of delivery.
- **Test strategy covers invariants, external behavior, convergence, and failures.**
  Pass. Contract tests, deterministic planning tests, and convergence/apply reporting tests will be updated to validate the new schema and continuity rules.
- **Modules are structured to be regenerable from specs and tests.**
  Pass. The feature is centered on explicit schema types, renderers, contract docs, and behavioral tests.

## Project Structure

### Documentation (this feature)

```text
specs/007-explainable-reconcile-interface/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   └── reconciliation-output.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
├── core/
└── io/

tests/
├── integration/
├── unit/
└── fixtures/
```

**Structure Decision**: Use the existing single-project Rust CLI layout. The feature primarily affects [src/core/types.rs](/home/outergod/code/github.com/outergod/core-ops/src/core/types.rs), [src/core/planner.rs](/home/outergod/code/github.com/outergod/core-ops/src/core/planner.rs), [src/cli/report.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs), [src/cli/plan.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/plan.rs), [src/cli/apply.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/apply.rs), relevant state/provenance helpers in [src/io/state.rs](/home/outergod/code/github.com/outergod/core-ops/src/io/state.rs), and contract/integration tests under [tests/integration](/home/outergod/code/github.com/outergod/core-ops/tests/integration).

## Phase 0 Research Summary

- The current deterministic plan JSON can be replaced in place, but only after enriching planner-facing data so the new schema is not inferred lossy from renderer strings.
- Existing persisted provenance and deterministic state files should remain stable in this feature; the migration target is the operator-facing plan/apply/result/explain contract, not on-disk state schemas.
- The new public contract should be represented by dedicated operator-facing Rust types, while existing deterministic planner, graph, drift, and convergence types remain internal inputs.
- Contract tests and legacy contract documentation must be updated in the same feature scope so no contradictory machine-readable schema remains normative after release.

## Phase 1 Design Summary

- Add operator-facing machine-readable types for `ManagedObjectRef`, `RevisionContext`, `Cause`, `DependencyEdge`, `SemanticDiff`, and the plan/apply/result/explain view shapes, with `RevisionContext` explicitly carrying `requested_repository`, `requested_ref`, `target_revision`, `last_applied_revision`, `last_applied_requested_repository`, `last_applied_requested_ref`, and `change_revision` so mutable desired-state selection context remains distinct from immutable reconciliation identity for both current and prior applied revisions.
- Keep `DeterministicReconciliationPlan`, `SemanticDependencyGraph`, `StructuredDriftRecord`, and persisted state records as internal planning/provenance types.
- Introduce view-building logic that converts deterministic planner and convergence outputs into full-scope entries, structured causes, dependency relations, direct-versus-transitive dependency distinctions, and layered summaries.
- Preserve the human-supplied repository location and requested ref as operator-facing provenance in plan/apply/result/explain views when available, including the selector context associated with the last successfully applied immutable revision, while ensuring comparison, convergence, and rollback semantics continue to use only the resolved immutable revision.
- Refine plan-entry action semantics so `update` represents an object's own material desired-state change, `restart` represents runtime reactivation driven by changed prerequisites or inputs in the current planned change set, and `recover` represents runtime corrective intent for an object whose declarative definition is unchanged but whose actual runtime state is not converged.
- Plan building must incorporate runtime verification or equivalent convergence context when deriving `recover` intent so unchanged declarative objects that still require corrective action are not misclassified as `no_op`.
- Replace the current plan JSON renderer with the target `PlanOutput` shape and extend apply/result/explain rendering to use the same authoritative model, including full-scope `ResultOutput` entries.
- Update human-readable rendering so it is a deterministic projection of the new machine-readable data rather than an independently shaped report, using an `object [action]` primary line, `because ...` explanation line, readable dependency tree, and diff evidence as supporting detail.
- Treat the humane-apply amendment as authoritative for User Story 2: default human apply output is concise and operator-oriented, verbose mode adds phases and expanded diagnostics, and structured mode emits only machine-readable events with no human formatting.
- Encode apply visibility and state semantics so unchanged objects are not misreported as skipped, default human output foregrounds acted-on, failed, and blocked objects, and raw JSON or provenance payloads never appear in human modes.
- Replace old contract assertions and supersede the legacy structured diff contract document with the new reconciliation output contract.
- Pin compatibility-sensitive serializer behavior with contract tests covering field names, enum values, deterministic array ordering, and absent-versus-null optional-field semantics.

## Post-Design Constitution Check

- **Functional core vs. imperative shell**: Still passes. View-model construction and rendering remain pure; no new side effects are introduced.
- **Declarative state model**: Improved. The design adds explicit public schema types rather than exposing ad hoc JSON objects.
- **Observability**: Improved. The design centers plans, phases, object outcomes, causes, dependencies, and provenance as typed contract data.
- **Compatibility discipline**: Managed. The design treats the schema replacement as an explicit compatibility event and confines stability-sensitive persisted state to existing schemas.
- **Versioning and provenance**: Preserved with an explicit dual-provenance model. Public outputs now need to distinguish mutable repository/ref selection context from immutable resolved reconciliation identity while leaving persisted provenance state structurally stable.
- **Test contract**: Stronger. The design requires contract tests for new view shapes, parity tests for human/machine rendering, and continuity tests across plan/apply/result.

## Complexity Tracking

No constitution violations requiring justification.

## Version Review Record

- Compatibility review outcome: minor version review required.
- Reason: feature 007 replaces the operator-facing plan JSON contract in place,
  adds authoritative machine-readable apply/result/explain contracts, and
  changes default human-visible CLI rendering semantics.
- Package version outcome: bump controller package version from `0.5.0` to
  `0.6.0`.
