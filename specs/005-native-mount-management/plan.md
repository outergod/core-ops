# Implementation Plan: Native Mount Management

**Branch**: `005-native-mount-management` | **Date**: 2026-03-24 | **Spec**: `specs/005-native-mount-management/spec.md`
**Input**: Feature specification from `/specs/005-native-mount-management/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Add native mount management to CoreOps as a first-class reconciled artifact for host services. The design introduces named mount declarations, bounded target-path preparation, mount-aware service dependencies materialized directly into generated native units, verification and failure reporting for mount lifecycle, optional automount support only for explicitly declared network-backed mounts, and conservative removal semantics for previously managed mounts. Merged changes under this feature require release-version-policy review, with an expected controller package-version update from `0.3.0` to `0.4.0`.

## Technical Context

**Language/Version**: Rust (stable toolchain, edition 2021)  
**Primary Dependencies**: clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json, systemd native unit generation via existing CoreOps boundaries  
**Storage**: Files on disk for desired-state repository content, generated native unit files, and existing canonical status state under `/var/lib/core-ops/status.json`  
**Testing**: `cargo test` (unit + integration)  
**Target Platform**: Linux host, primarily Fedora CoreOS / systemd-managed environments  
**Project Type**: CLI + systemd service/timer agent  
**Performance Goals**: Planning remains operator-interactive for typical single-host configs with up to 10 managed mounts; reconciliation adds only bounded mount verification and dependency generation overhead relative to existing single-host flows  
**Constraints**: Remain systemd-native; no Kubernetes-style storage abstractions; no generic network share management beyond native unit semantics; bounded directory preparation only; idempotent reapply; explicit failure diagnostics; path-based plus explicit unit dependency materialization; conservative removal that fails explicitly if the mount remains busy  
**Scale/Scope**: Single-host reconciliation for mount-backed services with dozens of native artifacts and a small number of managed mounts per host

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; side effects remain isolated to filesystem preparation, generated unit writes, systemd reloads, and native mount activation/deactivation.
- Desired state, observed state, reconciliation plans, and outcomes remain explicit data structures, extended with named mount declarations and service-to-mount dependencies.
- Abstractions stay minimal by extending existing reconciliation and native-unit generation flows rather than introducing a parallel storage subsystem.
- Effects, assumptions, and failure modes remain explicit, including blocked services, degraded mount dependencies, and busy mount removals.
- Idempotence and convergence are preserved: unchanged desired state produces no unintended remounts or service churn; retries converge after transient mount recovery.
- Open standards and native interfaces are preferred through systemd `.mount` and `.automount` units plus native dependency directives.
- Observability covers diffs, generated native dependency semantics, activation and removal actions, failures, and audit/status output.
- Provenance and status surfaces continue to identify controller version, desired-state revision, and applied outcome in machine-readable form.
- Safe defaults are documented: ordinary mount units remain default, automount is opt-in and network-backed only, and mount removal does not force unmount destructive behavior silently.
- Compatibility impact is assessed: this feature changes externally observable reconciliation behavior and generated native units but stays within current desired-state model boundaries.
- Release version policy impact is assessed: the feature is a minor-version-review candidate and canonical controller version remains sourced from `Cargo.toml`.
- Test strategy covers invariants, external behavior, convergence, dependency generation, removal semantics, and failures.
- Modules remain regenerable from spec, research, and tests.

Status: PASS (pre-design). Post-design re-check: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/005-native-mount-management/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── mount-declaration.md
│   └── mount-removal.md
└── tasks.md
```

### Source Code (repository root)

```text
src/
├── cli/
│   ├── plan.rs
│   ├── apply.rs
│   ├── status.rs
│   ├── report.rs
│   └── args.rs
├── core/
│   ├── evaluate.rs
│   ├── planner.rs
│   ├── diff.rs
│   ├── reconcile.rs
│   ├── verify.rs
│   ├── validation.rs
│   ├── types.rs
│   └── unit.rs
└── io/
    ├── repo.rs
    ├── observed.rs
    ├── quadlet.rs
    ├── systemd.rs
    ├── apply.rs
    └── state.rs

tests/
├── integration/
│   ├── test_mount_contracts.rs
│   ├── test_mount_reconcile.rs
│   ├── test_mount_failures.rs
│   ├── test_mount_removal.rs
│   ├── test_mount_reuse.rs
│   ├── test_plan.rs
│   ├── test_reconcile_apply.rs
│   ├── test_unit_lifecycle.rs
│   ├── test_verification.rs
│   ├── test_ordering.rs
│   ├── test_idempotence.rs
│   └── test_config_cleanup.rs
└── unit/
    ├── test_types.rs
    ├── test_validation.rs
    ├── test_planner.rs
    ├── test_verification.rs
    └── test_evaluation_determinism.rs
```

**Structure Decision**: Keep the existing single-project Rust layout. Extend `core` with named mount declarations, dependency materialization, removal planning, and lifecycle verification; extend `io` with mount-aware repo loading, observation, and native systemd application boundaries; extend CLI/reporting surfaces to expose mount diffs and failure/removal results without creating a new subsystem.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |

## Version Review Record

- Trigger: this feature introduces new managed mount artifacts, new generated native dependency semantics, and new externally observable removal behavior.
- Compatibility policy review: minor version review required.
- Expected controller package version update: `0.3.0 -> 0.4.0`.
- Rationale: the change materially affects observable reconciliation behavior and managed artifacts, but it does not currently require a major compatibility break under the pre-1.0 controller policy.
