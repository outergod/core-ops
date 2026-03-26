# Implementation Plan: Deterministic Reconciliation

**Branch**: `006-deterministic-reconcile` | **Date**: 2026-03-25 | **Spec**: `specs/006-deterministic-reconcile/spec.md`
**Input**: Feature specification from `/specs/006-deterministic-reconcile/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Introduce deterministic reconciliation as an explicit CoreOps planning model built on normalized three-way comparison (`desired`, `last_applied`, `actual`), a minimal semantic dependency graph, dependency-aware rollback, bounded non-convergence detection, and one canonical structured diff model for both human and machine consumers. The implementation extends existing reconciliation, observation, state-persistence, and reporting flows rather than adding a separate subsystem, and it makes the successful apply boundary explicit: a revision becomes `last_applied` only after side effects complete and post-apply verification confirms convergence.

## Technical Context

**Language/Version**: Rust (stable toolchain, edition 2021)  
**Primary Dependencies**: Existing CoreOps Rust stack: clap, thiserror, miette, log, systemd-journal-logger, serde, serde_json, tempfile; systemd and Quadlet remain the runtime integration surfaces  
**Storage**: Files on disk for desired-state repository content and the canonical persisted CoreOps status or reconciliation snapshot state under the runtime state directory (currently centered on `/var/lib/core-ops/status.json`) with bounded retained successful snapshots for rollback eligibility  
**Testing**: `cargo test` (unit + integration)  
**Target Platform**: Linux host, primarily Fedora CoreOS / systemd-managed environments  
**Project Type**: Single-binary CLI reconciler with systemd-managed agent mode  
**Performance Goals**: Planning and dry-run diff generation remain operator-interactive for typical single-node scopes; repeated planning with identical inputs remains deterministic; retry behavior is bounded and never infinite  
**Constraints**: Single-node correctness first; no replacement of systemd runtime execution; explicit semantic dependency graph owned by CoreOps; canonical normalization per resource kind; bounded retry and bounded rollback history; machine-readable diff as the source of truth; successful apply boundary only after post-verify convergence; compatibility-conscious evolution of persisted state, CLI output, and reconciliation semantics  
**Scale/Scope**: Single-host managed scope covering generated systemd units, Quadlet resources, mount and automount resources, and rendered host artifacts, with dozens of managed objects and bounded retained rollback history per host

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit: normalization, three-way comparison, dependency graph construction, action classification, rollback planning, and convergence detection stay in `core`; filesystem, systemd, and persisted-state mutation stay in `io`.
- Desired, last applied, and actual state are first-class data models; plans, drift records, dependency edges, and convergence outcomes are explicit values rather than incidental control flow.
- Abstractions remain minimal: this feature extends existing `core`, `io`, and `cli` modules instead of creating a new architectural tier.
- Effects, assumptions, and failure modes are explicit in structured diff, rollback eligibility, non-convergence reporting, and successful-apply persistence rules.
- Idempotence and convergence strategy are defined: identical normalized inputs produce identical plans; retry is bounded; non-convergence is surfaced explicitly.
- Open standards and native interfaces remain primary: CoreOps plans semantics and persists state, while systemd and Quadlet remain runtime executors.
- Observability covers dry-run plans, structured diffs, dependency context, drift categories, rollback outcomes, and convergence diagnostics.
- Provenance and status surfaces continue to identify controller revision, desired revision, baseline revision, and reconcile outcome in machine-readable form.
- Safe defaults are preserved: blocked actions fail before unsafe mutation, rollback requires retained successful snapshots, and partial or failed reconciliations never advance `last_applied`.
- Compatibility impact is explicit: this feature materially changes reconciliation semantics, persisted state expectations, rollback behavior, and operator-visible output.
- Release version policy impact must be reviewed because persisted schema, CLI output, and reconciliation semantics become more expressive and externally observable.
- Test strategy covers deterministic planning, cycle detection, drift classification, rollback eligibility and execution, bounded retry, and machine-readable diff contracts.
- Modules remain regenerable from spec, research, contracts, and tests.

Status: PASS (pre-design). Post-design re-check: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/006-deterministic-reconcile/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── rollback.md
│   └── structured-diff.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── args.rs
│   ├── plan.rs
│   ├── apply.rs
│   ├── report.rs
│   ├── status.rs
│   └── agent.rs
├── core/
│   ├── diff.rs
│   ├── errors.rs
│   ├── planner.rs
│   ├── reconcile.rs
│   ├── retry.rs
│   ├── types.rs
│   ├── validation.rs
│   ├── verify.rs
│   └── unit.rs
└── io/
    ├── apply.rs
    ├── observed.rs
    ├── repo.rs
    ├── state.rs
    ├── systemd.rs
    └── audit.rs

tests/
├── integration/
│   ├── test_apply_report.rs
│   ├── test_deterministic_planning.rs
│   ├── test_plan.rs
│   ├── test_quickstart_validation.rs
│   ├── test_reconcile_apply.rs
│   ├── test_rollback.rs
│   ├── test_status_state.rs
│   ├── test_convergence.rs
│   ├── test_ordering.rs
│   ├── test_idempotence.rs
│   ├── test_retry.rs
│   └── test_verification.rs
└── unit/
    ├── test_invariants.rs
    ├── test_planner.rs
    ├── test_types.rs
    ├── test_validation.rs
    ├── test_verification.rs
    └── test_state_snapshot.rs
```

**Structure Decision**: Keep the existing single-project Rust layout. Extend `core` for normalized three-way planning, semantic dependency graphs, rollback eligibility, action classification, and convergence reasoning; extend `io` for observed-state loading, persisted retained snapshot handling, and apply-time result recording; extend `cli` for machine-readable and human-readable deterministic diff and rollback reporting.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | - | - |

## Version Review Record

- Trigger: this feature materially changes reconciliation semantics, structured plan and result output, rollback behavior, and persisted applied-state expectations.
- Compatibility policy review: completed during implementation validation.
- Outcome: bump controller package version from `0.4.0` to `0.5.0` because deterministic reconciliation changes externally observable reconciliation semantics, structured plan/apply output, rollback behavior, and the deterministic persisted-state contract.
- Canonical controller version remains sourced from `Cargo.toml`.
