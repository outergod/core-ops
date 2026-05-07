# Feature Specification: Adoption Readiness — README and Onboarding Experience

**Feature Branch**: `018-adoption-readiness`
**Created**: 2026-05-06
**Status**: Draft
**Input**: User description: "Increase adoption likelihood and onboarding experience by restructuring the README around operational comprehension, adding a 30-second mental model, an architecture diagram, an experiential walkthrough using `examples/03-immich`, an asciinema-based onboarding artifact, and elevated real-world examples — without changing CLI surface, behavior, or tone."

## Summary

After spec/017 landed in v2.2.0, the repository ships a stateless `--source-repo <PATH>` CLI surface and five real-world `examples/<NN-slug>/` directories. This feature spec turns those substrates into above-the-fold onboarding by restructuring the README around **operational comprehension** rather than philosophy-first exposition.

This is a **documentation-only** change. No CLI surface, no behavior, no schema, no tests change. Release intent is `patch` per the bump rules (no `src/` modifications).

## Clarifications

### Session 2026-05-06

- Q: Should the asciinema recording sanitize operator-private values (hostname, paths, domains, IPs)? → A: Yes — mirror spec/017 FR-009 — RFC 2606 reserved domains, RFC 5737 documentation IPs, generic prompt (e.g., `op@example`), project-relative paths, no real credentials or operator-private hostnames.
- Q: Is there a duration cap on the asciinema recording? → A: Hard cap ≤ 90 seconds. If unattainable, narrow demo scope (skip status, skip re-run, drop a service) rather than extend.
- Q: How faithful must the README walkthrough's plan-output block be to actual `core-ops plan` output? → A: Verbatim from a real run, with `...` elision permitted for repeats or uninteresting lines. No paraphrasing, no hand-tuning. Every non-elided line MUST appear byte-for-byte in actual output.
- Q: Who performs the dogfooding pass referenced by SC-011? → A: Author self-attestation only. The author re-reads the rendered README cold (no separate operator), captures takeaways verbatim in `synthesis.md`, and assesses pass/fail against US1's expectations. Self-knowledge of the changes is an acknowledged limit traded against feasibility.
- Q: What beats does the static walkthrough code block cover? → A: Plan output + a short idempotent re-run snippet showing the "no changes" line. Apply and status are carried only by the recording. Approximate budget: ~25 lines.

## Scope distinction

The proposal that triggered this spec asked, among other things, that the iteration "increase adoption likelihood." Adoption likelihood is unmeasurable inside this slice — there is no analytics, no funnel, no clone metric this spec can move. The spec therefore replaces "did adoption increase" with **structural** acceptance criteria (artifact existence, README ordering, line budget, stop-list absence) plus a single **dogfooding pass** (one operator unfamiliar with the changes describes the Git→host flow within 5 minutes). "Adoption" is the long-horizon hypothesis; "structural improvement" is what 018 commits to.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - First-time visitor mentally simulates a CoreOps workflow within 5 minutes (Priority: P1)

A technically experienced operator opens the GitHub README cold. Within roughly five minutes of reading, they can answer four questions without leaving the page:

1. *What is CoreOps?* (operating model — host-native, systemd/Quadlet, declarative reconciliation, Git-driven)
2. *What does using it feel like?* (a representative `core-ops plan` invocation and its output)
3. *Is it serious enough to evaluate?* (CI green, recent release, license clear)
4. *How does Git become host state?* (one diagram, one mental model)

**Why this priority**: This is the first-impression surface. Every other onboarding artifact is downstream of whether the operator continues past the README. If they bounce in 30 seconds because the page leads with philosophy, no walkthrough or example matters.

**Independent Test**: The author opens the rendered page on GitHub after restructure. Time-box: 5 minutes for the read. The author then writes down — verbatim, without re-consulting the README — what they took away in answer to "What does CoreOps do and what does running it look like?" The captured answer must reference (a) host-native systemd/Quadlet convergence, (b) the Git → core-ops → systemd flow, (c) at least one CLI command they saw, and (d) a sense of project credibility (badges, version, release cadence). Pass/fail recorded in `synthesis.md`. Self-knowledge of the changes is an acknowledged limit (Clarification 2026-05-06 Q4); the test is still useful as a forcing function for "did the README actually surface these signals on a cold read."

**Acceptance Scenarios**:

1. **Given** the rendered README, **When** the reader scrolls from top, **Then** the first non-title content they encounter is a row of trust badges (CI, E2E Gate, Latest Release, License), followed by a `## 30-second mental model` heading within the first 120 lines.
2. **Given** the rendered README, **When** the reader continues scrolling, **Then** they encounter an architecture diagram (rendered Mermaid block on GitHub) and a walkthrough section showing representative `core-ops plan` output, both before any installation instructions.
3. **Given** the rendered README, **When** the reader reaches the bottom, **Then** they have not encountered any of: hype-flagged terms (per FR-014 stop-list), third-party JS embeds, or marketing-style feature comparison matrices.

---

### User Story 2 - Operator sees what `core-ops plan` actually does before installing anything (Priority: P2)

An operator considering CoreOps wants concrete evidence that the tool produces deterministic, inspectable output before they download a binary or write a single config file. They scroll to a "What using CoreOps feels like" section and see (a) a static text block of representative plan output for a real example, and (b) a link to a recorded asciinema session of the same workflow end-to-end.

**Why this priority**: Static walkthroughs and live recordings are independent trust signals — the static blocks prove "the output is shaped like this" (plan) and "re-running is a no-op when state already matches" (idempotent re-run); the asciinema proves "a real session runs in this time, with this cadence, on this kind of host." P2 because P1 (mental model) gates whether the operator reads this section at all; without P1, P2 is invisible.

**Independent Test**: A reviewer reads the walkthrough section. They can (a) name two services that appear in the static plan-output block (the first of the two static blocks per FR-006), (b) follow the link to `docs/onboarding.cast` and play it back locally with `asciinema play`, (c) describe what idempotent re-run behavior looks like from the recording — and from the second static block (the "no changes" snippet) — without re-running the recording.

**Acceptance Scenarios**:

1. **Given** the README walkthrough section, **When** the reader inspects the static plan output block, **Then** the block is a fenced code block of representative `core-ops plan --source-repo examples/03-immich --host example` output containing recognizable units (e.g., `immich-server.container`, `immich-internal.network`).
2. **Given** the README walkthrough section, **When** the reader clicks the recording link, **Then** they reach `docs/onboarding.cast` (in-tree, valid asciicast v2 format).
3. **Given** the in-tree recording, **When** the reader runs `asciinema play docs/onboarding.cast` locally, **Then** the cast plays back end-to-end and exercises the `examples/03-immich` walkthrough including at least one idempotent re-run demonstrating no host mutation when state already matches.

---

### User Story 3 - Operator visualizes Git → host convergence without reading source code (Priority: P2)

An operator wants to understand how a Git repository becomes host state on a single page. They find a Mermaid block in the README that names: the Git source, `core-ops`, generated systemd/Quadlet units, the host, and the audit/status side outputs.

**Why this priority**: Architecture diagrams are the highest-bandwidth onboarding artifact when an operator's question is "where does my repository fit in this system?" Equal priority to US2 because the diagram and the walkthrough together answer the "what is this and what does it do" question; either alone leaves a gap.

**Independent Test**: A reviewer who has never used CoreOps reads the diagram. They can (a) name the four primary nodes (Git, core-ops, systemd/Quadlet, host), (b) identify audit/status as side outputs (dashed/secondary edges), (c) describe the data flow direction (Git → core-ops → host).

**Acceptance Scenarios**:

1. **Given** the rendered README on GitHub, **When** the reader views the Architecture section, **Then** a Mermaid diagram renders inline with at least four nodes including the substrings `Git`, `core-ops`, `systemd`.
2. **Given** the same diagram, **When** the reader follows directional edges, **Then** the primary flow is unambiguous (left-to-right or top-to-bottom) and audit/status side outputs are visually distinguished (e.g., dashed edges or secondary nodes).
3. **Given** a non-GitHub render context (raw markdown, a mirror without Mermaid support), **When** the diagram fails to render, **Then** the surrounding README prose names the same four nodes and the flow direction explicitly so the architecture is recoverable from text alone.

---

### User Story 4 - Future maintainer preserves the operational-first ordering across edits (Priority: P3)

A future contributor adds a new section to the README (e.g., a new badge, a new credibility signal, a configuration recipe). They consult `specs/018-adoption-readiness/checklists/readme-structure.md` and the spec's structural FRs. Their edit preserves the section ordering, badge row composition, and stop-list discipline.

**Why this priority**: Documentation drifts. Without an explicit structural contract, the next contributor adds a "Features" matrix back at the top, or sneaks "production-grade" into a section header, and the spec/018 work is undone within three months. P3 because the maintenance load is real but the immediate user is hypothetical.

**Independent Test**: A future contributor edits the README to add a section. The pre-merge checklist at `specs/018-adoption-readiness/checklists/readme-structure.md` catches any deviation from §FR-001 ordering, the §FR-002 badge row, or the §FR-014 stop-list.

**Acceptance Scenarios**:

1. **Given** the structural checklist, **When** a contributor proposes a README edit, **Then** the checklist names every constraint (ordering, badge row, line budget, stop-list, no third-party JS embeds) in a runnable form (`grep`, `wc`, heading inspection).

---

### Edge Cases

- **Mermaid renders inconsistently outside GitHub** (RSS, mirrors, terminal viewers like `glow`). → Mitigated by US3-AC-3: the surrounding prose names the same four nodes textually, so the architecture is recoverable from text alone.
- **The asciinema recording drifts from CLI output as `core-ops` evolves**. → Accepted bounded risk: a reviewer responsibility, not a CI gate. The `docs/onboarding-script.sh` regeneration entry point exists so re-recording is a one-command operation when output drifts.
- **README walkthrough block diverges from `examples/03-immich` plan output as the example evolves**. → Mitigated by FR-006 (verbatim-from-real-invocation rule plus `...` elision): the walkthrough block is regenerated from a real `core-ops plan` invocation against `examples/03-immich` at authoring time, and the polish phase of the implementation re-validates by re-running (per tasks.md Phase 7 / SC-007a).
- **GPU device passthrough in `examples/03-immich` cannot be exercised on the recording host**. → The recording script substitutes a placeholder device path or runs on a host with the correct device shape; documented in the script header.
- **Operator reads the README on a network that blocks asciinema.org embeds** (corporate firewall, RSS, terminal viewer). → FR-013 forbids third-party JS embeds entirely; the recording is served from the in-tree `.cast` file with a static text fallback in the README.
- **CHANGELOG.md or release notes drift between the README's badge claim and the actual published release**. → No new failure mode introduced by 018; the existing release governance owns CHANGELOG/version/tag consistency.
- **A future contributor adds a hype-flagged term** (e.g., "production-grade"). → Caught by FR-014 stop-list grep documented in `checklists/readme-structure.md`.

## Requirements *(mandatory)*

### Functional Requirements

#### README structure and ordering

- **FR-001**: The root `README.md` MUST follow this section ordering, top-to-bottom:
  1. Title block (logo + tagline) — preserved from current README lines 1–10.
  2. Badge row (CI, E2E Gate, Latest Release, License) — single line, no heading, immediately after title block.
  3. `## 30-second mental model` — concise operational framing, ≤ 200 words, covering host-native convergence, systemd/Quadlet centricity, declarative reconciliation, and Git-driven operation.
  4. `## Architecture` — Mermaid block depicting Git → core-ops → systemd/Quadlet → host with audit/status as side outputs, plus surrounding prose recoverable when Mermaid fails to render (per US3-AC-3).
  5. `## What using CoreOps feels like` — walkthrough using `examples/03-immich` and the post-017 `--source-repo` flag; embeds a static plan-output code block plus a link to `docs/onboarding.cast`.
  6. `## Real-world examples` — five-entry list/links pointing to `examples/<NN-slug>/README.md`.
  7. `## Quick start` — folds current "Installation (Current Phase)" + "First Interaction".
  8. `## Why CoreOps exists` — compressed to ≤ 15 lines.
  9. `## What CoreOps is not` — compressed to ≤ 12 lines.
  10. `## Trust and release model` — folds the existing Credibility *table* (artifacts, verification env), Minimal Trust Story, and Release & Verification Model into one section.
  11. `## AI authorship` — compressed to ≤ 12 lines.
  12. `## Target audience · License · Further reading` — final reference block.
- **FR-002**: The badge row MUST contain exactly four badges in this order: CI, E2E Gate, Latest Release, License. No other badges may be promoted to the top row in this slice.
- **FR-003**: The README MUST be ≤ 400 lines after restructure.

#### Onboarding artifacts

- **FR-004**: The README MUST contain a Mermaid fenced code block depicting the architecture flow, with at least four nodes including the substrings `Git`, `core-ops`, `systemd`. Audit/status side outputs MUST be visually distinguished from the primary flow (e.g., dashed edges, secondary node shape).
- **FR-005**: The README walkthrough section MUST use `examples/03-immich` as its canonical example and invoke `core-ops plan --source-repo examples/03-immich --host example` as the canonical command.
- **FR-006**: The README walkthrough section MUST contain **two** fenced code blocks: (1) a plan-output block from the canonical `core-ops plan --source-repo examples/03-immich --host example` invocation, and (2) a short re-run snippet showing the idempotent "no changes" output (e.g., a second `core-ops plan` invocation immediately after `core-ops apply` produces no actionable diff). Both blocks MUST be derived **verbatim from real invocations**: every non-elided line MUST appear byte-for-byte in actual command output. Paraphrasing and hand-tuning are forbidden. `...` elision lines (a single line containing only the literal `...` and optional surrounding whitespace) are permitted to omit repeated or uninteresting lines for readability. The plan-output block MUST contain at least one recognizable Quadlet unit identifier from `examples/03-immich/services/` (e.g., `immich-server.container`, `immich-internal.network`). The combined budget for both blocks SHOULD be ≤ ~25 lines (excluding fence markers); if the natural plan output exceeds this, elide aggressively rather than expand the budget. Apply and status outputs are carried by the recording (`docs/onboarding.cast`), not by static blocks.
- **FR-007**: The repository MUST publish an asciinema recording at `docs/onboarding.cast` exercising the single `core-ops apply --source-repo examples/03-immich --host example` invocation end-to-end. The recording is **apply-only** — the static plan-output and idempotent-re-run blocks in the README walkthrough section (per FR-006) are the canonical source of plan/re-plan content; including those beats in the recording adds duration without motion. The recording's total duration MUST be ≤ 90 seconds. If 90 seconds is insufficient, the scope MUST be narrowed (drop a service from the example, or use a smaller example) — the duration cap MUST NOT be extended. The repository MUST also publish a derived inline-renderable GIF sidecar at `docs/assets/core-ops-demo.gif`, produced by `agg` (asciinema-agg) from the same `.cast` source so the README walkthrough section can embed motion via standard Markdown image syntax. The GIF is a derived artifact (regenerated whenever the cast is re-recorded); the `.cast` remains the source-of-truth. The recording MUST be produced on a host where `core-ops` runs natively (not via SSH delegation): SSH transport collapses the line-by-line streaming-output timing that the recording is meant to capture, so SSH-delegated recording is explicitly out of spec.
- **FR-008**: The recording at `docs/onboarding.cast` MUST be a valid asciicast v2 file (JSON header on first line) playable with `asciinema play`.
- **FR-009**: The repository MUST publish a regeneration entry point at `docs/onboarding-script.sh` that is executable, has a shebang, contains the literal string `examples/03-immich`, and documents in its header the deterministic command sequence used to produce `onboarding.cast` AND the GIF sidecar at `docs/assets/core-ops-demo.gif`. The script MUST invoke `agg` after recording to render the GIF from the `.cast`. The script header documents the pinned versions of both `asciinema` and `agg` used at recording time.
- **FR-009a**: The asciinema recording at `docs/onboarding.cast` and the regeneration script at `docs/onboarding-script.sh` MUST mirror spec/017 FR-009's sanitization rule: no operator-private values may appear in the captured session or the script. Specifically:
  - Any hostname displayed in the shell prompt MUST be a generic placeholder (e.g., `example`, `op`, `host`); the operator's real machine hostname MUST NOT appear.
  - Any domain literal MUST use RFC 2606 reserved domains (`*.example.com`, `*.example.org`, `*.test`, `*.invalid`, `*.localhost`).
  - Any IP literal MUST use RFC 5737 documentation ranges (`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`).
  - Filesystem paths MUST be project-relative or use generic placeholders (e.g., `/home/op/...`); the operator's real home-directory path or other private paths MUST NOT appear.
  - No real credentials, tokens, secrets, or environment-variable values sourced from the operator's private setup MUST appear.

#### Tone and structure preservation

- **FR-010**: The README MUST preserve the existing sober, technically serious tone. The Constitution's Principle 1 (clarity, no hype) is the operating reference.
- **FR-011**: Philosophy content (current "Why CoreOps Exists", "What CoreOps Is Not", "AI Authorship", "Target Audience" sections) MUST remain inline in the README. Extraction to `docs/philosophy.md` or similar is **out of scope** for this slice.
- **FR-012**: The existing "Real-World Examples" section landed in v2.2.0 (current README lines 115–138) MUST be preserved in spirit but renamed to `## Real-world examples` and elevated above `## Quick start` per the FR-001 ordering. The five existing one-line example descriptions are preserved unchanged in content; only ordering changes.
- **FR-013**: The README MUST NOT include any third-party JavaScript embed, iframe, or `<script>` tag (asciinema.org or otherwise). Standard Markdown / HTML image embeds (e.g., `![alt](path.gif)` or `<img src="path.gif">`) of the in-tree GIF sidecar at `docs/assets/core-ops-demo.gif` are explicitly permitted — they render inline on GitHub and other Markdown viewers without JS. An optional clickable link to an asciinema.org-hosted upload is permitted as an alternative anchor (the recommended pattern is to wrap the GIF `<img>` in an `<a href="https://asciinema.org/a/<id>">` so a single click opens the full asciinema.org player); the asciinema.org SVG-badge embed pattern is also permitted (it's a static SVG image, not a script).
- **FR-014**: The README MUST NOT introduce any of the following hype-flagged terms (case-insensitive): `enterprise-ready`, `industry-leading`, `production-grade`, `🚀`. The stop-list is the falsifiable surrogate for "no hype-oriented language."

#### Spec Kit deliverables

- **FR-015**: The repository MUST publish the spec at `specs/018-adoption-readiness/` with: `spec.md` (this file), `plan.md`, `tasks.md`, `research.md`, `quickstart.md`, `synthesis.md` (post-implementation retrospective), and `checklists/readme-structure.md` (acceptance-criteria runbook). `data-model.md` and `contracts/` are explicitly **omitted** — no new data structures or CLI contracts.
- **FR-016**: The repository MUST publish a release-governance fragment at `changes/018-adoption-readiness.md` declaring `release_intent: patch`, `scope: docs`, with a one-line summary of the README restructure.

#### Negative requirements (out of scope)

- **FR-017**: This change MUST NOT modify any file under `src/`, `.github/workflows/`, `examples/`, or `LICENSE`. **`Cargo.toml` MAY be bumped to a patch version** when triggered by release-governance rules (specifically the `packaged_readme_surface` rule, which fires when `README.md` is modified because the README ships in release bundles per the Quick Start documentation). The bump is metadata-only and does not constitute a source-code change. `Cargo.lock` is regenerated automatically by cargo and follows the same metadata-only treatment. **Existing test fixtures and integration assertions under `tests/integration/` and `tests/fixtures/` MAY also be updated** when their assertions reference README section names (renamed by FR-001) or pin the controller version string (bumped per the carve-out above). Such updates are mechanical fixture maintenance: they propagate spec/018's structural rename into existing tests without introducing new test surface, new CI gates, or assertions exercising new behavior. The prohibition is preserved for **new tests, new fixtures, or assertions exercising new behavior** — none of those are introduced by spec/018.
- **FR-018**: This change MUST NOT introduce any new CLI surface, new flags, new commands, schema changes, or behavioral changes to existing commands.
- **FR-019**: This change MUST NOT add a CI lint step that enforces FR-014 (stop-list) or FR-003 (line budget) — these are reviewer-checked via `checklists/readme-structure.md`. CI surface area is preserved.

### Key Entities

- **README structural contract**: The ordering, badge row, line budget, and stop-list together define a structural contract for the README that is enforced via the checklist at `specs/018-adoption-readiness/checklists/readme-structure.md` rather than runtime tests.
- **Onboarding recording**: An asciicast v2 file at `docs/onboarding.cast` plus its regeneration entry point at `docs/onboarding-script.sh`. The pair is a single onboarding artifact: the cast is the immutable visible deliverable, the script is the maintainability backstop.
- **Mental model**: A ≤ 200-word prose section that compresses the operational claim of CoreOps (host-native, systemd/Quadlet, declarative reconciliation, Git-driven) into a form a technically experienced operator can absorb in one read.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: N/A — this feature ships only documentation and a recorded artifact. No core or shell code is added or modified.
- **Declarative state model**: The README itself is a declarative artifact: structural ordering and badge composition are declared in FR-001/FR-002, and a checklist verifies the declared shape. No runtime state model is touched.
- **Idempotence & convergence**: N/A — no convergence behavior changes. The asciinema recording demonstrates idempotent re-run of the existing `core-ops apply` semantics (per FR-007); it does not introduce new idempotence guarantees.
- **Explicit effects/failures**: N/A — no new failure modes introduced. Edge cases (Mermaid render failure, asciinema drift) are handled by existing recoverability (text fallback, regeneration script).
- **Observability**: N/A — no new observable runtime behavior. Documentation is the observed artifact.
- **Provenance & traceability**: N/A — no provenance impact. The README references the existing release model; CHANGELOG continuity is owned by the existing release governance, not by this spec.
- **Safe defaults**: N/A — no defaults change.
- **Compatibility**: All existing README links (CHANGELOG, CODE_OF_CONDUCT, docs/development.md, examples/) MUST continue to resolve unchanged after restructure. The existing `## Real-World Examples` section's content is preserved (FR-012); only ordering and heading style change.
- **Release version policy**: Documentation-only change with no `src/` modifications. Per the `changes/README.md` bump rules, this is `patch`. Declaring `patch` is at-or-above the inferred minimum.
- **Release intent artifact**: `changes/018-adoption-readiness.md` declares `release_intent: patch`, `scope: docs`.
- **Changelog discipline**: The fragment's `summary` field becomes the `[Unreleased]` bullet; the post-merge promote step in CI moves it into a tagged section automatically. No manual `CHANGELOG.md` editing.
- **Test contract**: No new test surface. The existing `cargo test` and `cargo clippy --all-targets -- -D warnings` gates remain authoritative for the codebase. The README structural contract is verified at PR-review time via `checklists/readme-structure.md`, not via CI. **VM-backed scenario assessment**: No new behavioral mutation classes. Per Principle 10's exemption clause, no new VM-backed scenario is required for this feature; this exemption is recorded explicitly here as the documented justification.
- **Regenerability**: The Mermaid block is text in the README and trivially regenerable. The asciinema recording is regenerable via `docs/onboarding-script.sh` (FR-009). The static walkthrough plan output is regenerable by re-running `core-ops plan --source-repo examples/03-immich --host example`.

> **Verification Guidance section omitted**: This feature is documentation-only and does not participate in spec-derived end-to-end verification. The README structural contract is enforced via review checklist, and the asciinema artifact's correctness is asserted at recording time and at re-recording time, not in CI. Removing the section per the template's instruction.

## Success Criteria *(mandatory)*

### Measurable outcomes

- **SC-001**: After this change merges, `wc -l README.md` MUST report ≤ 400 lines (per FR-003).
- **SC-002**: After this change merges, the rendered README on GitHub MUST display a row of exactly four badges (CI, E2E Gate, Latest Release, License) immediately after the title/logo block, before any heading (per FR-002).
- **SC-003**: After this change merges, the README MUST contain a `## 30-second mental model` heading within the first 120 lines (per FR-001 §3 and the mental-model entity definition).
- **SC-004**: After this change merges, the README MUST contain at least one ` ```mermaid ` fenced code block whose content includes the substrings `Git`, `core-ops`, and `systemd` (per FR-004).
- **SC-005**: After this change merges, the file `docs/onboarding.cast` MUST exist and be a valid asciicast v2 file (first line is a JSON header with `"version": 2`) (per FR-007/FR-008).
- **SC-005a**: After this change merges, the asciicast v2 header at the top of `docs/onboarding.cast` MUST report a `duration` field ≤ 90 (seconds) — measurable via `head -n 1 docs/onboarding.cast | jq '.duration'` (per FR-007).
- **SC-005b**: After this change merges, the file `docs/assets/core-ops-demo.gif` MUST exist as a non-empty GIF (first 6 bytes match `GIF89a` or `GIF87a`) and the README MUST embed it via standard image-tag syntax (`<img src="docs/assets/core-ops-demo.gif" ...>` or `![](docs/assets/core-ops-demo.gif)`) inside the `## What using CoreOps feels like` section. File size SHOULD be ≤ 1 MB (a soft cap; if exceeded, narrow the recording or tune `agg --idle-time-limit` rather than relax). Per FR-007/FR-013.
- **SC-006**: After this change merges, the file `docs/onboarding-script.sh` MUST exist, be executable, contain a shebang on line 1, and contain the literal string `examples/03-immich` somewhere in the body (per FR-009).
- **SC-006a**: After this change merges, `grep -i -E '(not\.one|ulthar|192\.168\.|10\.0\.|172\.16\.)' docs/onboarding.cast docs/onboarding-script.sh` MUST return zero matches; the recording and script MUST contain no operator-private domain markers, no RFC 1918 private IP ranges suggestive of a real homelab, and no operator-specific hostnames (per FR-009a). The same stop-list is applied to `docs/assets/core-ops-demo.gif` indirectly: the GIF is rendered from the same sanitized `.cast` source, so any string the cast does not contain cannot appear in the rendered frames.
- **SC-007**: After this change merges, the README walkthrough section MUST contain a fenced code block whose content includes at least one Quadlet unit identifier from `examples/03-immich/services/` (per FR-006).
- **SC-007a**: Each non-elided line in **both** README walkthrough code blocks (the plan-output block and the idempotent re-run snippet, excluding lines that are exactly `...` or whitespace) MUST be reproducible byte-for-byte from real `core-ops plan` / `core-ops apply` invocations against `examples/03-immich` on the post-018 master tree (per FR-006). Verified at PR review by re-running and spot-checking; not a CI gate.
- **SC-007b**: After this change merges, the README walkthrough section MUST contain exactly two fenced code blocks (one plan, one re-run-no-changes), and their combined non-blank line count SHOULD be ≤ ~25 lines (per FR-006).
- **SC-008**: After this change merges, `grep -i -E '(enterprise-ready|industry-leading|production-grade|🚀)' README.md` MUST return zero matches (per FR-014).
- **SC-009**: After this change merges, every link target the pre-018 README pointed to (LICENSE, CHANGELOG.md, CODE_OF_CONDUCT.md, docs/development.md, each `examples/<NN-slug>/`) MUST still resolve from the post-018 README (per Compatibility).
- **SC-010**: After this change merges, no file under `src/`, `.github/workflows/`, `examples/`, or `LICENSE` MUST have been modified by the diff (per FR-017). `Cargo.toml` MAY have a patch-version bump (e.g., `2.2.0` → `2.2.1`) driven by the `packaged_readme_surface` release-governance rule, and `Cargo.lock` is regenerated to match — both are metadata-only changes. **Existing test files under `tests/integration/` and `tests/fixtures/` MAY have mechanical maintenance updates** when README section names change (driven by FR-001) or when fixtures reference the bumped Cargo.toml version (driven by the `packaged_readme_surface` carve-out). No new test files, new fixtures, or new assertions exercising new behavior are introduced. Verified via `git diff master..HEAD --stat -- src/ .github/workflows/ examples/ LICENSE` returning empty.
- **SC-011**: A dogfooding pass MUST be recorded in `specs/018-adoption-readiness/synthesis.md` as **author self-attestation** (per Clarification 2026-05-06 Q4). The author opens the rendered README cold, time-boxed at 5 minutes for the read, then writes down — verbatim, without re-consulting the README — what they took away in answer to "What does CoreOps do and what does running it look like?" The captured answer is assessed pass/fail per US1's Independent Test. Self-knowledge is an acknowledged limit; the test is preserved as a forcing function. A failure does not block merge but MUST trigger a follow-up issue with concrete remediation proposals.
- **SC-012**: The synthesis section in `synthesis.md` MUST address the proposal's §9 questions: (a) did onboarding clarity materially improve, (b) what adoption/trust gaps remain, (c) does the repository now communicate the operational experience more effectively. Answers are operator-attested, not metric-driven.

## Assumptions

- The post-017 v2.2.0 master is the authoring baseline. `examples/03-immich/` and the `--source-repo` CLI surface are available and stable.
- The asciinema recording is authored once at implementation time on a host with appropriate device shape for `examples/03-immich` (or with a documented device-substitution in `docs/onboarding-script.sh`). Drift between the recording and CLI output is accepted as a bounded risk; re-recording is a one-command operation when needed.
- GitHub is the primary read surface for the README. Mermaid renders natively; non-GitHub render contexts are degraded but recoverable per US3-AC-3.
- The "Credibility" section's badges and table content land in their new positions (top badge row + Trust section) without losing information; no new artifacts beyond the License badge are added in this slice.
- The dogfooding operator (SC-011) is sourced informally from the operator's network; their response is captured verbatim in `synthesis.md` under a pseudonymous attribution if anonymity is preferred.
- Future iterations may revisit FR-011 (extraction of philosophy to `docs/philosophy.md`) once the compressed inline form has been observed in practice. This spec deliberately keeps philosophy inline.
- A future spec (019 or beyond) may introduce CONTRIBUTING.md or ARCHITECTURE.md based on the synthesis findings; those are out of scope here and flagged in `synthesis.md` if relevant.
