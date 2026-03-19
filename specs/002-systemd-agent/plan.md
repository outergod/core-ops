# Implementation Plan: Systemd-Managed Host Agent

**Branch**: `[002-systemd-agent]` | **Date**: 2026-03-19 | **Spec**: specs/002-systemd-agent/spec.md
**Input**: Feature specification from `/specs/002-systemd-agent/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Deliver a systemd-managed host agent that runs unattended via a oneshot service
triggered by a timer, emits journald audit events by default, and reconciles
container, socket, and volume Quadlet artifacts with explicit ordering and
verification. The agent preserves functional-core/imperative-shell boundaries,
explicit failure reporting, idempotence, and observability while staying within
native system primitives and avoiding generic host configuration management.

## Technical Context

**Language/Version**: Rust (stable toolchain)  
**Primary Dependencies**: Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger  
**Storage**: Files on disk (Quadlet unit files + optional reconciliation state)  
**Testing**: cargo test (unit + integration)  
**Target Platform**: Fedora CoreOS (single host)  
**Project Type**: CLI + systemd service/timer agent  
**Performance Goals**: 95% of scheduled runs finish within 2 minutes for up to 50 artifacts  
**Constraints**: No host configuration management beyond Quadlet/systemd/container scope; explicit failures; journald observability  
**Scale/Scope**: Single host, tens of artifacts, no fleet orchestration

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
specs/002-systemd-agent/
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

**Structure Decision**: Single project with `core` (pure planning/verification),
`io` (Git/systemd/Quadlet side effects), and `cli` (entrypoints and reporting).
Systemd service/timer definitions live in `specs/002-systemd-agent/contracts/`.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |
