# Quickstart: What spec/018 adoption-readiness changes

**Feature**: 018-adoption-readiness
**Status**: Draft (post-clarify)
**Audience**: an operator who has read the post-018 `README.md` and wants a one-page reference for what changed and where the new artifacts live.

## TL;DR

Spec/018 is a **documentation-only** iteration. It restructures `README.md` around operational comprehension and adds two artifacts under `docs/` to anchor the onboarding experience. **No CLI changes. No behavior changes. No schema changes.**

## What's new on disk

| Path | Purpose |
|---|---|
| `README.md` | Restructured per `spec.md` FR-001 (12 sections, ≤ 400 lines). Top of file: 4 badges (CI, E2E Gate, Latest Release, License) → 30-second mental model → architecture (Mermaid) → walkthrough → real-world examples → install → philosophy → trust → AI authorship → footer. |
| `docs/onboarding.cast` | Asciicast v2 recording, ≤ 90 s, of an end-to-end `examples/03-immich` walkthrough including an idempotent re-run. Sanitized — no operator-private hostnames, paths, domains, IPs, or credentials. |
| `docs/onboarding-script.sh` | Executable shell script documenting the deterministic command sequence used to produce `onboarding.cast`. Pins the `asciinema` version used at recording time. |
| `changes/018-adoption-readiness.md` | Release fragment (`release_intent: patch`, `scope: docs`). Validates as exempt under `always_exempt_documentation_or_formatting`. |
| `specs/018-adoption-readiness/` | Spec scaffold — `spec.md`, `plan.md`, `research.md`, this file, `synthesis.md` (post-implementation), `tasks.md` (produced by `/speckit.tasks`), `checklists/readme-structure.md`. |

## What's not new on disk

Confirmed unchanged by the 018 diff:

- `src/`, `Cargo.toml`, `Cargo.lock`, `tests/`, `.github/workflows/`, `examples/`, `LICENSE`.
- All existing CLI surfaces (`core-ops plan|apply|explain|status|init`, `core-ops-verify`, `core-ops-release`).
- `CHANGELOG.md` — re-rendered by `core-ops-release changelog --write`, but the change-log machinery itself is untouched.

## How to verify the change locally

```bash
# 1. Structural — README shape and stop-list
wc -l README.md                                               # ≤ 400
grep -c '^## 30-second mental model' README.md                # 1
grep -c '```mermaid' README.md                                # ≥ 1
grep -ciE '(enterprise-ready|industry-leading|production-grade|🚀)' README.md  # 0

# 2. Onboarding artifacts
test -f docs/onboarding.cast && head -n 1 docs/onboarding.cast | jq '.version'  # 2
head -n 1 docs/onboarding.cast | jq '.duration'               # ≤ 90
test -x docs/onboarding-script.sh && grep -F examples/03-immich docs/onboarding-script.sh

# 3. Sanitization stop-list
grep -iE '(not\.one|ulthar|192\.168\.|10\.0\.|172\.16\.)' docs/onboarding.cast docs/onboarding-script.sh  # exit 1 (no matches)

# 4. Walkthrough fidelity (reviewer manual step)
core-ops plan --source-repo examples/03-immich --host example  # spot-check non-elided README lines

# 5. Asciinema playback
asciinema play docs/onboarding.cast                           # plays end-to-end ≤ 90 s

# 6. Release governance
cargo run --bin core-ops-release -- validate --base-ref master  # passes (exempt)
```

## How to re-record the asciinema cast

If `core-ops` CLI output drifts and the recording becomes stale:

```bash
docs/onboarding-script.sh
```

The script documents the recording shell (clean `env`, generic `PS1`, placeholder paths) and the `asciinema` version pin. The output is a fresh `docs/onboarding.cast`. Commit it.

## How spec/018 relates to spec/017

Spec/017 (v2.2.0) shipped the *substrate*: the stateless `--source-repo` CLI surface and five real-world `examples/<NN-slug>/` directories. Spec/018 turns that substrate into above-the-fold onboarding. The 018 walkthrough is the canonical tour of the 017 surface.

If you only read one example: `examples/03-immich`. It is what the 018 walkthrough demonstrates and what `docs/onboarding.cast` records.

## Where to look next

- `specs/018-adoption-readiness/spec.md` — FRs / SCs / Edge Cases / Clarifications.
- `specs/018-adoption-readiness/plan.md` — technical approach, constitution check, structure decision.
- `specs/018-adoption-readiness/research.md` — Phase 0 decisions (asciinema tooling, Mermaid render, sanitization tooling, fidelity verification).
- `specs/018-adoption-readiness/tasks.md` — implementation task breakdown (produced by `/speckit.tasks`; not present in this commit).
- `specs/018-adoption-readiness/synthesis.md` — post-implementation retrospective (produced in the implementation phase; not present in this commit).
