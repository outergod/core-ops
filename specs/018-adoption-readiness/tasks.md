---
description: "Task list for 018-adoption-readiness implementation"
---

# Tasks: Adoption Readiness — README and Onboarding Experience

**Input**: Design documents from `/specs/018-adoption-readiness/`
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `quickstart.md`. `data-model.md` and `contracts/` are intentionally absent (per spec FR-015).

**Tests**: **Exempted from new authorship.** Spec/018 is documentation-only. FR-019 explicitly forbids adding a CI lint step that enforces structural rules; no new tests, fixtures, or CI assertions are introduced. **Mechanical maintenance** of existing integration tests and fixtures IS permitted under FR-017's carve-out when assertions reference renamed README section names (driven by FR-001) or pin the bumped controller version string (driven by the `packaged_readme_surface` Cargo.toml bump). Verification of the structural contract itself is reviewer-driven via `specs/018-adoption-readiness/checklists/readme-structure.md` (T020 below). The standard `cargo test` and `cargo clippy --all-targets -- -D warnings` gates remain authoritative for the codebase.

**Organization**: Tasks are grouped by user story. US1 (P1) is the MVP — restructured README with placeholders for the artifacts US2 and US3 contribute. US2 and US3 are independent fills of the placeholders. US4 codifies the structural contract for future maintainers.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Maps the task to a user story (US1, US2, US3, US4) — required for user-story-phase tasks only
- File paths in descriptions are absolute or repo-root-relative

## Path conventions

Single Rust crate at repository root. This iteration touches:

- `README.md` (modified)
- `Cargo.toml`, `Cargo.lock` (patch bump 2.2.0 → 2.2.1, governance-driven; per FR-017 carve-out)
- `docs/onboarding.cast`, `docs/onboarding-script.sh` (new)
- `specs/018-adoption-readiness/checklists/readme-structure.md`, `specs/018-adoption-readiness/synthesis.md` (new)
- `tests/integration/test_distribution_*.rs`, `tests/fixtures/distribution/entrypoint-snapshot.md`, `tests/fixtures/provenance_state/valid-success.json` — mechanical maintenance updates only (FR-001 section renames + Cargo.toml version bump; per FR-017 carve-out)
- `CHANGELOG.md` (auto-rendered by `core-ops-release changelog --write`)

No `src/`, `.github/workflows/`, `examples/`, or `LICENSE` modifications.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold the spec directory's optional artifacts that later phases populate.

- [X] T001 [P] Create `specs/018-adoption-readiness/checklists/` directory and an empty `readme-structure.md` skeleton (header + table-of-checks placeholder). Populated by T020 in Phase 6.
- [X] T002 [P] Create `specs/018-adoption-readiness/synthesis.md` skeleton with frontmatter + section headers for "Dogfooding pass", "What materially improved", "Remaining adoption/trust gaps", "Operational communication effectiveness" (proposal §9 questions). Populated by T025 in Phase 7.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Build the asciinema regeneration entry point that US2's recording task depends on.

**⚠️ CRITICAL**: T003 must complete before T014 (recording the cast) in Phase 4.

- [X] T003 Author `docs/onboarding-script.sh`: executable shell script with shebang `#!/usr/bin/env bash`, version-pinned `asciinema` invocation in the header (per research.md R1, FR-009), env-scrubbed shell launcher (`env -i HOME=/home/op PATH=... TERM=xterm-256color PS1='op@example $ ' bash --noprofile --norc -c '...'` per research.md R4), deterministic command sequence exercising `core-ops plan --source-repo examples/03-immich --host example` plus apply + idempotent re-run, and the literal string `examples/03-immich` in the body (FR-009, SC-006). `chmod +x docs/onboarding-script.sh`.

**Checkpoint**: Foundation ready — Phase 3 (US1 MVP) and Phase 5 (US3) can now begin in parallel.

---

## Phase 3: User Story 1 — First-time visitor mentally simulates within 5 minutes (Priority: P1) 🎯 MVP

**Goal**: A technically experienced operator reading the README cold can answer four questions within ~5 minutes: what CoreOps is, what running it looks like, whether it is serious, and how Git becomes host state. Achieved by restructuring sections into operational-first order, promoting trust signals to the top, and inserting a 30-second mental model section.

**Independent Test**: Open the rendered `README.md` on GitHub. Within the first 120 lines (excluding badges and title), the reader encounters: badge row → mental model heading → architecture placeholder → walkthrough placeholder → real-world examples link list. Stop-list grep returns 0 matches. `wc -l README.md` ≤ 400.

> **No new tests authored** — per FR-019 (no new CI lint surface). Structural verification is via T020 checklist + T021/T022 polish checks. (Existing integration tests may receive mechanical fixture-maintenance updates per the FR-017 carve-out when README section names change.)

### Implementation for User Story 1

- [X] T004 [US1] Reorder all top-level sections in `README.md` per spec FR-001 (the canonical 12-section ordering: Title → Badge row → 30-second mental model → Architecture → What using CoreOps feels like → Real-world examples → Quick start → Why CoreOps exists → What CoreOps is not → Trust and release model → AI authorship → Target audience · License · Further reading). Move "Real-World Examples" (currently at lines 115–138) to its new position above "Quick start" per FR-012; preserve its content unchanged.
- [X] T005 [US1] Promote badge row to immediately after the title block in `README.md` (FR-002, SC-002). Add the missing License badge: `[![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue)](LICENSE)`. Final badge row contains exactly four badges in order: CI, E2E Gate, Latest Release, License. Remove the orphaned "Credibility" heading (badges no longer live under it).
- [X] T006 [US1] Author the `## 30-second mental model` section in `README.md` (≤200 words, per FR-001 §3, SC-003). Cover: host-native convergence, systemd/Quadlet centricity, declarative reconciliation, Git-driven operation. Section heading must appear within the first 120 lines.
- [X] T007 [US1] Compress philosophy sections in `README.md` to declared budgets per FR-001: "Why CoreOps exists" (≤15 lines), "What CoreOps is not" (≤12 lines), "AI authorship" (≤12 lines).
- [X] T008 [US1] Fold the existing "Credibility" *table* (artifacts row + verification environment row) + "Minimal Trust Story" + "Release & Verification Model" sections into a single `## Trust and release model` section in `README.md` per FR-001 §10. Preserve the verification-environment row's identifier (`fedora-coreos-self-hosted@2026-04-fcos`) verbatim.
- [X] T009 [US1] Insert anchor placeholders in `README.md` for Phase 4 and Phase 5 to fill: a `## Architecture` heading with a one-line note (filled by US3 T017/T018) and a `## What using CoreOps feels like` heading with a one-line note (filled by US2 T013/T016). The placeholders are valid markdown but explicitly note "[populated by US2/US3 implementation]" so the MVP is parseable as-is.
- [X] T010 [US1] Verify intermediate state of `README.md` after T004–T009: `wc -l README.md` ≤ 400 (SC-001); `grep -c '^## 30-second mental model' README.md` returns 1 (SC-003); `grep -ciE '(enterprise-ready|industry-leading|production-grade|🚀)' README.md` returns 0 (SC-008). Fix any failure before proceeding to Phase 4/5.

**Checkpoint**: US1 MVP complete. The README is structurally restructured even without US2's walkthrough or US3's Mermaid block. Architecture and Walkthrough sections are placeholders.

---

## Phase 4: User Story 2 — Operator sees what `core-ops plan` does before installing (Priority: P2)

**Goal**: A first-time reader sees concrete, deterministic CLI output evidence in the README walkthrough section before installing anything. The static plan-output and idempotent re-run blocks come from real invocations; the asciinema recording carries apply and status motion.

**Independent Test**: README walkthrough section contains exactly two fenced code blocks (one plan output, one re-run "no changes"), combined ≤ ~25 lines, every non-elided line reproducible from a real `core-ops plan` invocation. `docs/onboarding.cast` exists, asciicast v2 header reports duration ≤ 90 s, sanitization stop-list grep returns 0.

> **Depends on**: T003 (regeneration script must exist before T014 can record).

### Implementation for User Story 2

- [X] T011 [P] [US2] Run `core-ops plan --source-repo examples/03-immich --host example` against post-018 master tree on a clean host (no prior apply against `--host example`); capture output verbatim into a working buffer for T013. Note the recognizable Quadlet unit identifiers from `examples/03-immich/services/` (e.g., `immich-server.container`, `immich-internal.network`, `immich-public.network`, `immich-database.container`, `immich-redis.container`, `immich-ml.container`, `traefik-edge.container`) for reference under SC-007.
- [X] T012 [US2] Run `core-ops apply --source-repo examples/03-immich --host example` on a sanitized test host, then run `core-ops plan --source-repo examples/03-immich --host example` a second time; capture the idempotent "no changes" output (per FR-006 second block). Output goes to a working buffer for T013. **Not marked [P]**: shares the `--host example` state with T011 — apply mutates host state that T011's pre-apply plan reads. Run T012 after T011 completes (sequential), or use a fresh isolated host to safely parallelize.
- [X] T013 [US2] Author the `## What using CoreOps feels like` section in `README.md` (replacing the placeholder T009 inserted): one short intro paragraph naming the canonical command; first fenced code block from T011 (plan output, elide repeats with `...` lines, keep recognizable unit identifiers); second fenced code block from T012 (idempotent re-run); both blocks verbatim from real invocations per FR-006. Combined non-blank line count SHOULD be ≤ ~25 (SC-007b). No paraphrasing.
- [X] T014 [US2] Execute `docs/onboarding-script.sh` (T003) to record `docs/onboarding.cast`. Verify with `head -n 1 docs/onboarding.cast | jq '.version'` returns `2` (SC-005), `head -n 1 docs/onboarding.cast | jq '.duration'` returns ≤ 90 (SC-005a), and `asciinema play docs/onboarding.cast` plays end-to-end. If duration > 90 s, narrow the demo scope per FR-007 (skip status, skip re-run, drop a service) — DO NOT extend the cap.
- [X] T015 [US2] Run sanitization stop-list grep: `grep -iE '(not\.one|ulthar|192\.168\.|10\.0\.|172\.16\.)' docs/onboarding.cast docs/onboarding-script.sh` MUST return zero matches (SC-006a, FR-009a). If matches found, re-record with a cleaner shell environment (env-scrub hostname, paths, any leaked private values).
- [X] T016 [US2] Append a link to `docs/onboarding.cast` from the walkthrough section in `README.md` (e.g., `**Recording**: [docs/onboarding.cast](docs/onboarding.cast) — play locally with \`asciinema play\``). Confirm no `<script>` tag, no `<iframe>`, and no `https://asciinema.org/...js` reference in the README diff (FR-013).

**Checkpoint**: US2 complete. Walkthrough section is fully populated with verbatim output blocks plus the in-tree recording link. Static fallback covers determinism + idempotence even for readers without asciinema tooling.

---

## Phase 5: User Story 3 — Operator visualizes Git → host convergence (Priority: P2)

**Goal**: A reader on GitHub sees a Mermaid block depicting the high-level flow (Git → core-ops → systemd/Quadlet → host) with audit/status as side outputs. A reader on a non-Mermaid renderer can recover the same architecture from the surrounding prose.

**Independent Test**: README contains a `mermaid` fenced code block with ≥ 4 nodes including the substrings `Git`, `core-ops`, `systemd`. The surrounding `## Architecture` prose names the same four nodes and the flow direction in plain text (per US3-AC-3). Mermaid block renders correctly on the GitHub PR preview.

> **Independent of US2** — can run in parallel with Phase 4 if labor available.

### Implementation for User Story 3

- [X] T017 [US3] Author the Mermaid block in `README.md` `## Architecture` section (replacing the placeholder T009 inserted). Suggested structure (matching the recommended preview from clarification):
  ```mermaid
  flowchart LR
    GIT[Git repository<br/>services/ + hosts/]
    CORE[core-ops<br/>plan / apply / explain]
    STATE[systemd + Quadlet units<br/>generated state]
    HOST[host<br/>systemd-managed services]
    AUDIT[(audit + status<br/>JSON snapshot)]
    GIT --> CORE
    CORE --> STATE
    STATE --> HOST
    CORE -.-> AUDIT
    HOST -.-> AUDIT
  ```
  Verify FR-004: ≥ 4 nodes, contains substrings `Git`, `core-ops`, `systemd`. Audit/status edges use dashed `-.->` syntax to signal side-output status.
- [X] T018 [US3] Author 2–4 lines of prose in `## Architecture` immediately around the Mermaid block (per US3-AC-3): name the four nodes (`Git repository`, `core-ops`, `systemd + Quadlet units`, `host`), the side-output (`audit + status`), and the flow direction (left-to-right) in plain text. The architecture must be recoverable when Mermaid fails to render.
- [X] T019 [US3] Push the branch and open a draft PR; verify on the GitHub PR preview that the Mermaid block renders as a diagram (not as a raw fenced code block). Adjust syntax if rendering fails (e.g., move `<br/>` placement, simplify node labels). Manual visual verification — not a CI gate.

**Checkpoint**: US3 complete. The Mermaid diagram renders on GitHub; the prose covers non-Mermaid renderers.

---

## Phase 6: User Story 4 — Future maintainer preserves operational-first ordering (Priority: P3)

**Goal**: A future contributor adding a section to the README has a runnable structural checklist that catches deviations from FR-001 ordering, FR-002 badge row, FR-003 line budget, FR-013 third-party-JS prohibition, FR-014 stop-list, FR-009a sanitization rule.

**Independent Test**: A reviewer running every command in `specs/018-adoption-readiness/checklists/readme-structure.md` against the post-implementation tree gets all-pass results.

### Implementation for User Story 4

- [ ] T020 [US4] Populate `specs/018-adoption-readiness/checklists/readme-structure.md` (skeleton from T001) with one runnable check per FR/SC: line-budget (`wc -l README.md` ≤ 400), badge-row composition (4 badges, no others promoted), mental-model heading existence + position, Mermaid block existence + node substrings, walkthrough two-block + line budget, sanitization stop-list grep, hype stop-list grep, link-target resolve check (every pre-018 link still resolves), no third-party JS embed (`grep -E '(asciinema\.org/.*\.js|<iframe|<script)' README.md` returns 0). Each check labeled with the FR/SC it verifies.

**Checkpoint**: US4 complete. Structural contract is documented and runnable.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Final structural validation, dogfooding pass, synthesis writing, release-governance verification.

- [ ] T021 [P] Final structural pass: `wc -l README.md` ≤ 400 (SC-001); `grep -c '^## 30-second mental model' README.md` = 1 (SC-003); `grep -c '```mermaid' README.md` ≥ 1 (SC-004).
- [ ] T022 [P] Final stop-list pass: `grep -ciE '(enterprise-ready|industry-leading|production-grade|🚀)' README.md` = 0 (SC-008).
- [ ] T023 [P] Final link-resolve pass: confirm every pre-018 README link target still resolves — `LICENSE`, `CHANGELOG.md`, `CODE_OF_CONDUCT.md`, `docs/development.md`, each `examples/<NN-slug>/README.md` for NN ∈ {01,02,03,04,05} (SC-009). Use a one-liner: `grep -oE '\]\(([^)]+)\)' README.md | sed -E 's/\]\(|\)//g' | xargs -I{} test -e {} || echo "{} missing"`.
- [ ] T024 [P] Final no-source-touched pass: `git diff master..HEAD --stat -- src/ .github/workflows/ examples/ LICENSE` returns empty (SC-010, FR-017). `Cargo.toml`, `Cargo.lock`, and `tests/` are excluded from this check per the FR-017 carve-out (governance-driven version bump + mechanical fixture maintenance for FR-001 section renames).
- [ ] T025 Dogfooding pass per SC-011 / FR Clarification Q4: after a cooling-off period of ≥ 24 h since the last `README.md` edit, open the rendered `README.md` on the GitHub PR preview cold. Time-box: 5 minutes for the read. Then write down — verbatim, without re-consulting the README — what was taken away in answer to "What does CoreOps do and what does running it look like?" Capture the answer in `specs/018-adoption-readiness/synthesis.md` under the "Dogfooding pass" heading. Assess pass/fail per US1's Independent Test (must reference: host-native systemd/Quadlet convergence; Git → core-ops → systemd flow; ≥ 1 CLI command seen; sense of project credibility from badges/version).
- [ ] T026 Author the body of `specs/018-adoption-readiness/synthesis.md` (skeleton from T002) per SC-012: address proposal §9 questions — (a) did onboarding clarity materially improve, (b) what adoption/trust gaps remain, (c) does the repository now communicate the operational experience more effectively. Operator-attested answers; reference T025 outcome explicitly.
- [ ] T027 Run `cargo run --bin core-ops-release -- validate --base-ref master` from repo root; confirm `Outcome: passed` and `Classification: exempt` with `Applied Rules: always_exempt_documentation_or_formatting`. If `CHANGELOG aligned: no`, run `cargo run --bin core-ops-release -- changelog --write` and re-validate.
- [ ] T028 Open the implementation PR; the description references this `tasks.md`, links the rendered Mermaid block (already verified in T019), and links `docs/onboarding.cast` for reviewer playback. Confirm CI green (no Rust changes, so `ci.yml` is fast-path; the release-governance step passes per T027).

---

## Dependencies & Execution Order

### Phase dependencies

- **Phase 1 (Setup)**: No dependencies — T001 and T002 can start immediately and run in parallel.
- **Phase 2 (Foundational)**: T003 depends on Phase 1 completion (script lives alongside the spec scaffold). Blocks T014 in Phase 4 (recording requires the script).
- **Phase 3 (US1, P1, MVP)**: Depends on Phase 2 completion. T004–T009 are sequential (all touch `README.md`); T010 depends on T004–T009.
- **Phase 4 (US2)**: Depends on Phase 3 completion (walkthrough placeholder must exist before T013 fills it). T011 and T012 are sequential by default (T011 before T012) since they share `--host example` state on a single host; only safely parallel on isolated hosts. T013 depends on both. T014 depends on T003 + T013. T015 depends on T014. T016 depends on T015.
- **Phase 5 (US3)**: Depends on Phase 3 completion (architecture placeholder must exist). T017 → T018 → T019 sequential.
- **Phase 6 (US4)**: Depends on Phase 1 completion (skeleton from T001) and ideally on Phase 3/4/5 (so the checklist can encode the actual final structure). Recommend running after Phase 4 and Phase 5 land.
- **Phase 7 (Polish)**: Depends on all prior phases. T021–T024 can run in parallel. T025 has a 24-hour cool-off prerequisite. T026 depends on T025. T027 depends on the final commit. T028 depends on T027.

### User story dependencies

- **US1 (P1)** restructures the README and inserts placeholders. Must complete before US2 / US3 fill the placeholders.
- **US2 (P2)** and **US3 (P2)** are independent of each other once US1 is done — can run in parallel.
- **US4 (P3)** is independent of US2/US3 in spirit but the checklist's runnable commands assume the final shape, so author after US1/US2/US3 land.

### Parallel opportunities

- T001 ∥ T002 (different files in `specs/018-adoption-readiness/`).
- ~~T011 ∥ T012~~ — sequential by default (shared `--host` state); only safely parallel on isolated hosts.
- US2 (Phase 4) ∥ US3 (Phase 5) once Phase 3 lands.
- T021 ∥ T022 ∥ T023 ∥ T024 (independent grep / wc / link checks).

---

## Implementation Strategy

### MVP first (US1 only)

1. Phase 1 (T001, T002) → spec scaffold.
2. Phase 2 (T003) → asciinema regeneration script.
3. Phase 3 (T004–T010) → README structurally restructured with placeholders.
4. **STOP and VALIDATE**: T010 confirms ≤ 400 lines, mental model heading present, no stop-list terms.
5. Open as a draft PR. The MVP is shippable: badges promoted, mental model present, examples elevated, philosophy compressed. Architecture and walkthrough are explicit placeholders.

### Incremental delivery

After MVP:

1. Land US3 (Phase 5: T017–T019) → Mermaid renders on GitHub PR preview.
2. Land US2 (Phase 4: T011–T016) → walkthrough blocks populated, recording committed.
3. Land US4 (Phase 6: T020) → checklist for future maintainers.
4. Polish (Phase 7: T021–T028) → final structural pass, dogfooding, synthesis, validate, PR open.

### Solo author strategy

This iteration is single-author. The dogfooding step (T025) is self-attestation per Clarification Q4 — no external operator required. Schedule the 24-hour cool-off explicitly: complete T021–T024 on day N, sleep, run T025 on day N+1.

---

## Notes

- **No new tests authored** per FR-019. The verification surface is `specs/018-adoption-readiness/checklists/readme-structure.md` (T020), not CI. Existing integration tests and fixtures may receive mechanical maintenance updates per the FR-017 carve-out when README section names change (FR-001) or when fixtures pin the bumped Cargo.toml version (`packaged_readme_surface` carve-out).
- **No Rust source changes** → no `cargo test` / `cargo clippy` tasks specific to spec/018. Existing gates remain authoritative; if mechanical fixture maintenance lands, run `cargo test` and `cargo clippy --all-targets -- -D warnings` to confirm the codebase remains green.
- **Release governance**: `release_intent: patch` in `changes/018-adoption-readiness.md` (already created). Validates as exempt under `always_exempt_documentation_or_formatting`. CHANGELOG.md re-rendered via `core-ops-release changelog --write` and re-validated by T027.
- **`Cargo.toml` patch bump (2.2.0 → 2.2.1) is required** by the `packaged_readme_surface` release-governance rule because `README.md` ships in published release bundles (see `decision_018-packaged-readme-surface-cargo-bump`). FR-017 was relaxed mid-implementation to permit this.
- **No VM-backed scenario**: explicitly exempted in spec/plan Constitution Alignment under Principle 10 (no externally visible host behavior change).
- Update task checkboxes (`- [ ]` → `- [X]`) as work lands per memory feedback (`feedback_speckit_tasks_checklist.md`): per-task, not batched at session end.
- Commit per logical group (one task or one tightly-coupled cluster). Conventional commit format `docs(spec/018): ...`.
