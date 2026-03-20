# Implementation Plan: Layered Overrides for Reusable Desired State

**Branch**: `[003-layered-overrides]` | **Date**: 2026-03-20 | **Spec**: specs/003-layered-overrides/spec.md
**Input**: Feature specification from `/specs/003-layered-overrides/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Add a deterministic evaluation phase that composes shared base artifacts,
host-selected services, host-specific drop-ins, and bounded config payloads,
producing a concrete desired state before diff/plan/apply. Reuse is achieved
via native Quadlet and systemd drop-in semantics plus whole-file config layering,
without any templating language, and with explicit failures for invalid overlays
or undefined services.

## Technical Context

**Language/Version**: Rust (stable toolchain)
**Primary Dependencies**: Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger, serde, serde_yaml, libc
**Storage**: Files on disk (repository layout + bounded config payloads + evaluated desired state in memory)
**Testing**: cargo test (unit + integration)
**Target Platform**: Fedora CoreOS (single host)
**Project Type**: CLI + systemd service/timer agent
**Performance Goals**: Evaluation overhead <= 1s per 50 artifacts (per SC-004)
**Constraints**: No templating language; no semantic config merging; deterministic evaluation; explicit failure diagnostics; native Quadlet/systemd drop-ins only
**Scale/Scope**: Single host, shared base definitions reused across multiple hosts

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- Functional core and imperative shell boundaries are explicit; side effects are isolated.
- Desired/observed state, reconciliation plans, and outcomes are represented as data.
- Abstractions are minimal and justified; complexity tracking added if needed.
- Effects, assumptions, and failure modes are explicit in interfaces and returns.
- Idempotence and convergence strategy are defined, including retry behavior.
- Open standards and native interfaces are preferred; deviations justified.
- Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs.
- Safe defaults are documented; destructive actions require explicit intent.
- Compatibility impact is assessed; breaking changes are documented with migration.
- Test strategy covers invariants, external behavior, convergence, and failures.
- Modules are structured to be regenerable from specs and tests.

Status: PASS (pre-design). Post-design re-check: PASS.

## Project Structure

### Documentation (this feature)

```text
specs/003-layered-overrides/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── cli/
├── core/
└── io/

tests/
├── integration/
└── unit/
```

**Structure Decision**: Single project with `core` (evaluation, planning, validation), `io` (repo/systemd/Quadlet interactions), and `cli` (entrypoints/reporting). No additional services are introduced.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |
