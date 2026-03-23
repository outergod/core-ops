# Implementation Plan: Provenance and Reconciliation Revision Tracking

**Branch**: `004-reconcile-provenance` | **Date**: 2026-03-23 | **Spec**: `specs/004-reconcile-provenance/spec.md`
**Input**: Feature specification from `/specs/004-reconcile-provenance/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Add a canonical persisted provenance snapshot for CoreOps that records controller identity, desired-state observation, and the latest reconciliation outcome. The implementation adds a validated snapshot state model, atomic persisted-state updates, explicit never-run/in-progress/success/failed semantics, and mirrored CLI/status surfaces without introducing independent history or a second authoritative state source.

## Technical Context

**Language/Version**: Rust (stable toolchain, edition 2021)
**Primary Dependencies**: clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json
**Storage**: Files on disk under a runtime state directory for canonical persisted provenance; optional repository cache remains separate and non-authoritative
**Testing**: `cargo test` (unit + integration)
**Target Platform**: Linux host, primarily Fedora CoreOS / systemd-managed environments
**Project Type**: CLI + systemd service/timer agent
**Performance Goals**: Persist/read provenance snapshots with negligible overhead relative to reconciliation; status reads should remain effectively instantaneous for single-host operator use
**Constraints**: Complete-snapshot readability; atomic reader-visible updates; invalid/partial/unsupported state treated as absent; derivative local state only; no bounded history journal; current state + last outcome only; CLI/log views must mirror canonical file contents
**Scale/Scope**: Single-host provenance snapshot per CoreOps-managed host; one canonical status file and mirrored interfaces

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; side effects are isolated.
- Desired/observed state, reconciliation plans, and outcomes are represented as data.
- Abstractions are minimal and justified; complexity tracking added if needed.
- Effects, assumptions, and failure modes are explicit in interfaces and returns.
- Idempotence and convergence strategy are defined, including retry behavior.
- Open standards and native interfaces are preferred; deviations justified.
- Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs.
- Provenance and status surfaces identify reconciler revision, desired-state revision,
  and applied outcome in machine-readable form.
- Safe defaults are documented; destructive actions require explicit intent.
- Compatibility impact is assessed; breaking changes are documented with migration.
- Test strategy covers invariants, external behavior, convergence, and failures.
- Modules are structured to be regenerable from specs and tests.

Status: PASS (pre-design). Post-design re-check: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/004-reconcile-provenance/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli-status.md
│   └── status-file.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── args.rs
│   ├── status.rs
│   └── common.rs
├── core/
│   ├── types.rs
│   ├── reconcile.rs
│   ├── audit.rs
│   └── errors.rs
└── io/
    ├── audit.rs
    ├── apply.rs
    ├── lock.rs
    ├── repo.rs
    ├── observed.rs
    └── state.rs          # new persisted provenance boundary

tests/
├── integration/
│   ├── test_agent_service.rs
│   ├── test_reconcile_provenance.rs
│   ├── test_reboot_recovery.rs
│   ├── test_status_contract.rs
│   └── test_status_state.rs
└── unit/
    ├── test_types.rs
    ├── test_invariants.rs
    └── test_state_snapshot.rs
```

**Structure Decision**: Keep the existing single-project Rust layout. Extend `core` with provenance state types and transition rules, add a new `io/state.rs` boundary for canonical persisted snapshot reads/writes, and extend `cli` to expose mirrored status output from the canonical file.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |
