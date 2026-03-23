# Implementation Plan: GitOps Quadlet Controller

**Branch**: `[001-gitops-quadlet-controller]` | **Date**: 2026-03-18 | **Spec**: specs/001-gitops-quadlet-controller/spec.md
**Input**: Feature specification from `/specs/001-gitops-quadlet-controller/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Deliver the smallest viable controller for a single Fedora CoreOS host that
still enforces explicit reconciliation phases (plan → apply → verify), strong
idempotence, dry-run, and safe apply. The MVP is a CLI with an optional
long-running mode, uses only native primitives (Git, Quadlet, systemd), and
defaults auditability to structured systemd journal events. Rich artifacts such
as plans/reports are printed or explicitly exported on demand. The controller
MUST accept Git URLs (SSH/http(s)) or local paths as desired-state sources.
Applying state always implies regenerating systemd units (daemon-reload) and
activating/deactivating plus starting/stopping units as required by the plan.
Apply output MUST include a rich, operator-facing report (diffs/actions), not
just a short summary, consistent with plan output behavior.
Enablement is expressed via Quadlet [Install] sections; the controller MUST NOT
invoke systemctl enable/disable on generated services.
Architectural risk is minimized by keeping the core pure and limiting moving
parts, while deferring fleet, secrets, and host config.

## Technical Context

**Language/Version**: Rust (stable toolchain)  
**Primary Dependencies**: Git (CLI, URL clone/fetch), systemd (systemctl), Podman/Quadlet generator, clap, thiserror, miette  
**Storage**: Files on disk (Quadlet unit files + reconciliation state cache); audit defaults to systemd journal  
**Testing**: cargo test; integration tests validate repository and plan/apply flows without requiring live systemd  
**Target Platform**: Fedora CoreOS (single host)  
**Project Type**: CLI (with optional long-running loop)  
**Performance Goals**: Dry-run plan under 30 seconds for standard workload change; converge within 2 reconciliation cycles for valid changes  
**Constraints**: Immutable OS boundaries; no host mutation outside Quadlet/systemd/container scope; explicit failure handling; safe defaults  
**Scale/Scope**: Single host, tens to low hundreds of workloads; no fleet features in MVP

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
specs/001-gitops-quadlet-controller/
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

**Structure Decision**: Single project with three modules only: `core` (pure
planning/validation), `io` (filesystem/systemd/Git interactions), and `cli`
(entrypoints). Avoid extra layers until required.

## MVP Risk Minimization

- Keep the reconcile loop simple: validate → plan → apply → verify.
- Avoid background daemons by default; support a periodic loop only if needed.
- Use filesystem state and systemd queries; no external services or databases.
- Default audit sink is systemd journal; rich artifacts are ephemeral unless exported.
- Use clap derive for CLI parsing, thiserror for typed errors, and miette for
  user-facing diagnostics without coupling core logic to CLI types.
- Ignore dotfiles; warn (but do not fail) on unsupported file extensions when
  loading desired state.
- Default to read-only plan; apply requires explicit operator intent.
- Defer parallelism and concurrency controls unless required for correctness.

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| | | |
