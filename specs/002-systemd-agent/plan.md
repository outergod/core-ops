# Implementation Plan: Systemd-Managed Host Agent

**Branch**: `[002-systemd-agent]` | **Date**: 2026-03-19 | **Spec**: specs/002-systemd-agent/spec.md
**Input**: Feature specification from `/specs/002-systemd-agent/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Deliver a systemd-managed host agent that runs unattended via a oneshot service
triggered by a timer, emits journald audit events by default, and reconciles
container, socket, and volume Quadlet artifacts with explicit ordering and
verification. Generated units are not enabled/disabled by the controller; Quadlet
[Install] semantics govern enablement. The agent preserves functional-core/
imperative-shell boundaries, explicit failure reporting, idempotence, and
observability while staying within native system primitives and avoiding generic
host configuration management.

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

## Declarative State Model

Desired state is sourced from the Git repository’s Quadlet definitions, observed
state is derived from the host’s systemd-managed artifacts, and reconciliation
plans represent ordered actions required to converge. Runs persist their plan,
actions, and outcomes as explicit data structures surfaced through audit output.

## Idempotence Strategy

Reconciliation is designed to be safe to repeat: applying the same desired state
does not introduce additional changes, and repeated runs converge to the same
observed outcome. The plan phase determines no-op results when desired and
observed state already align, and apply actions are constructed to be stable on
subsequent executions.

## Phases

1. **Setup**: Ship systemd unit templates and deployment guidance.
2. **Foundational**: Add socket/volume types, ordering, verification model, and run lock.
3. **User Story 1 (MVP)**: Unattended agent entrypoint, CLI wiring, journald audit, lock usage.
4. **User Story 2**: Reconcile socket + volume artifacts end-to-end and report artifact types.
5. **User Story 3**: Verification checks wired through reconcile and audit outputs.
6. **Polish**: Documentation updates, performance/idempotence checks, targeted refactors.

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

## Compatibility Impact

No breaking changes expected for existing container-only workflows; new artifact
types and agent automation are additive.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |
