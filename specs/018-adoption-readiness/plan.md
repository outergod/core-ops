# Implementation Plan: Adoption Readiness — README and Onboarding Experience

**Branch**: `018-adoption-readiness` | **Date**: 2026-05-06 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/018-adoption-readiness/spec.md`

## Summary

After spec/017 landed in v2.2.0, the repository ships a stateless `--source-repo` CLI surface and five real-world `examples/<NN-slug>/` directories. This plan turns those substrates into above-the-fold onboarding by restructuring `README.md` around operational comprehension, embedding a Mermaid architecture diagram, adding a 30-second mental model, recording a sanitized ≤ 90 s asciinema demo of `examples/03-immich`, and tightening (without extracting) philosophy sections. **Documentation-only**; no `src/`, `Cargo.toml`, `tests/`, schema, or CLI changes. `data-model.md` and `contracts/` are explicitly omitted — no new structures or interfaces.

## Technical Context

**Language/Version**: N/A — no source code added or modified. Existing Rust 2021 toolchain in repository remains untouched.
**Primary Dependencies**: Mermaid (GitHub-native rendering for the in-README diagram); `asciinema` CLI (operator-side, version-pinned in `docs/onboarding-script.sh`) for recording the `.cast` artifact. **No new runtime dependencies.**
**Storage**: N/A — no persisted state added or read.
**Testing**: No `cargo test` cases added. Validation is structural and runs at PR review time via `specs/018-adoption-readiness/checklists/readme-structure.md` (grep / wc / file-existence checks). Existing `cargo test` and `cargo clippy --all-targets -- -D warnings` gates remain authoritative for the codebase and remain unchanged.
**Target Platform**: GitHub-rendered Markdown (primary read surface); local terminal via `asciinema play` for the recording playback.
**Project Type**: Documentation iteration on an existing CLI/agent project. No new codebase produced.
**Performance Goals**:
- `README.md` ≤ 400 lines after restructure (FR-003, SC-001).
- `docs/onboarding.cast` ≤ 90 s duration (FR-007, SC-005a).
- `## 30-second mental model` heading reachable within first 120 lines (SC-003).
**Constraints**:
- Stop-list: no `enterprise-ready`, `industry-leading`, `production-grade`, `🚀` (FR-014, SC-008).
- No third-party JS embeds in README (FR-013).
- No operator-private values in recording or script (FR-009a, SC-006a).
- Walkthrough plan-output blocks are verbatim from real invocations with `...` elision only (FR-006, SC-007a).
- All pre-018 README link targets MUST still resolve (Compatibility, SC-009).
**Scale/Scope**: Single `README.md` rewrite; two new files under `docs/` (`onboarding.cast`, `onboarding-script.sh`); one new release fragment under `changes/`; spec scaffold under `specs/018-adoption-readiness/`. No code or test files modified.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| 1. Functional Core, Imperative Shell | **N/A** — no code added. |
| 2. Declarative State as the Source of Truth | The README structural contract (ordering, badge row, line budget, stop-list) is itself declarative; FR-001/FR-002/FR-003/FR-014 declare the shape, the checklist verifies it. No runtime state model touched. |
| 3. Simplicity Over Cleverness | No new abstractions, no new tooling pipelines, no CI lint added (FR-019 explicitly forbids it). The reproduction script for the asciinema cast is shell with version-pinned tooling — minimum viable. |
| 4. Explicit Effects and Explicit Failure | Edge cases (Mermaid render failure, asciinema drift, third-party block, GPU passthrough) are surfaced in spec.md `Edge Cases`. No silent recovery introduced. |
| 5. Idempotence and Convergence First | **N/A** for the docs change itself. The recording demonstrates the existing apply idempotence (per FR-007); it does not introduce new convergence guarantees. |
| 6. Open Standards and Native Interfaces First | Mermaid and asciinema are open formats. asciicast v2 is a documented JSON schema. GitHub-native rendering is preferred over third-party embeds (FR-013). |
| 7. Observability as a Core Feature | The walkthrough block + recording **is** the observability surface for first-time readers. Existing CLI observability is unchanged. |
| 8. Safe Defaults, Explicit Power | **N/A** — no defaults change. |
| 9. Conservative Public Evolution | All pre-018 README link targets MUST still resolve (SC-009). The `Real-World Examples` section landed in v2.2.0 is preserved in spirit; only ordering and heading style change (FR-012). |
| 10. Tests Define the Contract | No new tests. No externally visible host behavior change → **VM-backed scenario exemption** is recorded explicitly here: this is documentation-only with zero CLI / behavior / schema impact, so per Principle 10's exemption clause, no new VM scenario is required. The exemption is also recorded in spec.md Constitution Alignment section. |
| 11. Regenerability Over Incidental Craftsmanship | The Mermaid block is text in the README; trivially regenerable. The asciinema cast is regenerable via `docs/onboarding-script.sh` (FR-009). The static walkthrough plan-output is regenerable from a real `core-ops plan` invocation (FR-006). |
| 12. Provenance, Versioning, and Behavioral Traceability | **N/A** for the docs change itself. The README will continue to surface release/version provenance (badges, CHANGELOG link). |
| 13. Release Governance and Intent | Documented and enforced. `changes/018-adoption-readiness.md` declares `release_intent: patch`, `scope: docs`. Validates as exempt under `always_exempt_documentation_or_formatting` per `core-ops-release validate`. CHANGELOG.md re-rendered via `core-ops-release changelog --write`. |

**Result**: PASS. No violations. No `Complexity Tracking` entries required.

## Project Structure

### Documentation (this feature)

```text
specs/018-adoption-readiness/
├── plan.md                           # This file (Phase 0/1 outputs combined)
├── spec.md                           # Feature spec (Session 2026-05-06 clarifications)
├── research.md                       # Phase 0 — research log (asciinema tooling, Mermaid render, line budget)
├── quickstart.md                     # Phase 1 — operator pointer for what 018 changes
├── synthesis.md                      # Phase 5 (post-implementation) — author self-attestation
├── checklists/
│   └── readme-structure.md           # Acceptance-criteria runbook (PR review)
└── tasks.md                          # Phase 2 output (created later by /speckit.tasks)
```

`data-model.md` is **omitted** — spec FR-015 explicitly states no new data structures are introduced.
`contracts/` is **omitted** — spec FR-015 explicitly states no new CLI contracts are introduced.

### Source artifacts (repository root)

```text
README.md                             # Largest single diff — restructured per spec FR-001
CHANGELOG.md                          # Auto-rendered by `core-ops-release changelog --write`
changes/
└── 018-adoption-readiness.md         # Release fragment (release_intent: patch, scope: docs)
docs/
├── onboarding.cast                   # New — asciicast v2 recording, ≤ 90 s, sanitized
└── onboarding-script.sh              # New — regeneration entry point + tooling pinning
```

**Not touched**: `src/`, `Cargo.toml`, `Cargo.lock`, `tests/`, `.github/workflows/`, `examples/`, `LICENSE`, `CODE_OF_CONDUCT.md`, `docs/development.md`, `docs/follow-ups.md`, `docs/core-ops.svg`, all existing `docs/*.md` proposal files.

**Structure Decision**: Single-project Rust crate with adjunct documentation tree. This feature adds files only under `docs/`, `changes/`, and `specs/018-adoption-readiness/`, plus modifies `README.md` and (auto-rendered) `CHANGELOG.md`. No `src/` or `tests/` paths are touched.

## Phase 0 — Research

The detailed Phase 0 output is recorded in [`research.md`](./research.md). Summary of resolved unknowns:

- **R1: Asciinema tooling and version pinning** — Decision recorded. Rationale: stable v2 schema, documented format, in-tree `.cast` artifact, no third-party JS embed required.
- **R2: Mermaid GitHub render fidelity** — Decision recorded. Rationale: GitHub-native render path is the primary read surface; non-GitHub fallback is text prose per spec US3-AC-3.
- **R3: README size benchmarks for similarly scoped infrastructure projects** — Decision recorded. Rationale: 400-line cap is empirically attainable per the post-spec/017 baseline (275 lines current).
- **R4: Recording sanitization tooling** — Decision recorded. Rationale: shell-environment substitution at recording time (`PS1`, `PWD`, env scrub) is sufficient; no post-processing of the `.cast` JSON is required.
- **R5: Walkthrough block fidelity verification** — Decision recorded. Rationale: spot-check at PR review by re-running `core-ops plan`; not a CI gate (per FR-019).

No `NEEDS CLARIFICATION` markers remain after Session 2026-05-06 (5 questions, all resolved in spec.md `## Clarifications`).

## Phase 1 — Design & Contracts

### data-model.md

**Skipped per spec FR-015.** No new data structures. The README itself is treated as a declarative artifact whose shape is defined by spec.md FR-001/FR-002/FR-003/FR-014 (ordering, badge row, line budget, stop-list). No `data-model.md` file is produced.

### contracts/

**Skipped per spec FR-015.** No new CLI surface, schema, API, or interface contract is introduced. The existing CLI surface from spec/017 is referenced by the walkthrough but not modified. No `contracts/` directory is produced.

### quickstart.md

Generated at [`quickstart.md`](./quickstart.md). Targets an operator who has read the post-018 README and wants a one-page reference for what this iteration changed and where the new artifacts live. Not a tutorial; a pointer.

### Agent context update

Run `.specify/scripts/bash/update-agent-context.sh claude` once plan.md and research.md are in place. This re-renders the auto-generated portion of `CLAUDE.md` to reflect any new technologies added by this plan. Spec 018 introduces no new technologies (only Mermaid + asciinema, both consumed externally), so the diff is expected to be limited to the `Recent Changes` and `Active Technologies` lines.

## Phase 2 — Tasks (NOT produced by this command)

Task breakdown is the output of `/speckit.tasks`. The implementation plan in `~/.claude/plans/based-on-the-actual-virtual-waterfall.md` (created during plan-mode entry) sketched a 25-task breakdown (T001–T025) across 5 phases (Spec scaffold; Content authoring; README restructure; Asciinema; Polish + synthesis). That sketch is a hint, not a contract — `/speckit.tasks` will author the canonical `tasks.md` from this plan and the spec.

## Constitution re-check (post-design)

Re-evaluating after Phase 0/1 produced research.md and quickstart.md (no other Phase 1 artifacts):

| Principle | Re-assessment |
|---|---|
| 3. Simplicity Over Cleverness | Confirmed — no new tooling pipelines, no CI lint, no post-processing of asciicast JSON. |
| 9. Conservative Public Evolution | Confirmed — research.md R3 captured the existing-link-resolve constraint; SC-009 covers it. |
| 10. Tests Define the Contract | Confirmed — VM-backed scenario exemption explicitly recorded. No new unit/integration tests. |
| 13. Release Governance and Intent | Confirmed — fragment + CHANGELOG re-render already validated as exempt by `core-ops-release validate`. |
| All others | Unchanged from pre-Phase 0 evaluation; no design decisions altered constitution alignment. |

**Result**: PASS. Plan is ready for `/speckit.tasks`.

## Complexity Tracking

*No Constitution Check violations. No entries required.*
