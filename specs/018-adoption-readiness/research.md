# Phase 0 Research: Adoption Readiness — README and Onboarding Experience

**Feature**: 018-adoption-readiness
**Date**: 2026-05-06
**Owner**: spec-018 implementation

## Scope

Resolve unknowns surfaced during plan authoring before Phase 1 design begins. All decisions here back the FRs and SCs in `spec.md`. No `NEEDS CLARIFICATION` markers remain after Session 2026-05-06.

---

## R1 — Asciinema tooling and version pinning

**Decision**: Use `asciinema` v2.x as the recording tool, asciicast v2 as the recorded format. Pin the recorder version in `docs/onboarding-script.sh` header and assert it at re-record time.

**Rationale**:
- Asciicast v2 is the documented, stable schema; the first line of the `.cast` file is a JSON header containing `"version": 2`, plus `width`, `height`, `timestamp`, `duration`, `command`, `title`, `env` fields (per asciinema/asciinema spec). SC-005 and SC-005a verify shape and duration via that header.
- v2 is supported by all current playback tooling: the `asciinema play` CLI, asciinema.org's hosted player, and third-party offline players. v3 exists but is opt-in via `--cols` / `--rows` and not the default — sticking with v2 maximizes compatibility.
- Recording in v2 keeps `docs/onboarding.cast` greppable and diffable as plain text JSON-lines, satisfying the "no binary artifact" preference of the project.
- Version pinning in the script header (e.g., `# asciinema --version: 2.4.0`) makes drift visible. CI does not enforce it (per spec FR-019).

**Alternatives considered**:
- **Plain `script(1)` recording with a typescript file**: rejected. Output is byte-stream; no time-stamping or playback semantics. Loses the "live demo" affordance.
- **terminalizer**: rejected. Node.js dependency, custom YAML format, no GitHub-native render path.
- **agg → animated SVG**: rejected as primary format (kept as optional sidecar in research.md follow-up — see R5). Adds a binary-ish artifact and a build step.
- **asciicast v3**: rejected for now. Not yet ubiquitous in playback tooling; stable v2 minimizes friction.

---

## R2 — Mermaid GitHub render fidelity

**Decision**: Author the architecture diagram as a fenced ` ```mermaid ` block embedded in `README.md`. Accept that non-GitHub render contexts (RSS, mirrors, terminal viewers like `glow`) may not render the block.

**Rationale**:
- GitHub renders Mermaid blocks natively in `.md` files since Feb 2022. No build step, no checked-in SVG, no external CI step. The diagram is text in the source — trivially regenerable per Constitution Principle 11.
- The substring requirements (`Git`, `core-ops`, `systemd` per FR-004) are verifiable by `grep` over `README.md`. SC-004 codifies this.
- Non-GitHub renderers degrade to "raw fenced code block" which is still legible: the block contains node labels in plain text. Spec US3-AC-3 mandates surrounding prose recoverable from text alone, so degradation does not break the architecture explanation.

**Alternatives considered**:
- **Hand-drawn SVG in `docs/architecture.svg`**: rejected. Binary-ish XML artifact, requires editing in Inkscape/Figma, drift risk if model changes. Added tooling for marginal visual gain. Constitution Principle 3 (Simplicity) prefers Mermaid.
- **ASCII art block**: rejected. Cluttered for ≥ 4 nodes; renders inconsistently across fonts.
- **External diagrams.net link**: rejected. Off-repo dependency; violates spirit of "primary artifacts in tree."

**Verification at PR time**: Open the GitHub PR diff page, confirm the Mermaid block renders. Manual; not a CI gate.

---

## R3 — README size benchmarks

**Decision**: Cap `README.md` at ≤ 400 lines after restructure (FR-003, SC-001). The cap is aspirational-but-firm; if compression cannot fit the content, escalate to the user before extracting philosophy to `docs/`.

**Rationale**:
- Current post-017 `README.md` is 275 lines. Adding the Mermaid block (~15 lines), 30-second mental model (~25 lines), walkthrough section with two code blocks (~30 lines), and the License badge (~1 line) totals ~70 lines of additions. Compression of "Why CoreOps Exists", "What CoreOps Is Not", and "AI Authorship" sections (per FR-001 §8/§9/§11) reclaims ~30 lines. Net: ~315 lines. Headroom of ~85 lines.
- 400 lines is the upper bound observed in similarly scoped infrastructure-tooling READMEs that still load cleanly on GitHub mobile (one mid-length scroll). Beyond 400, the page transitions from "scan-and-decide" to "reference document," which contradicts US1's 5-minute mental simulation goal.
- Cap is verified by `wc -l README.md` (SC-001). Falsifiable.

**Alternatives considered**:
- **No cap**: rejected. Enables drift over time; the structural contract is meaningless without a numeric target.
- **300-line cap**: rejected. Forces philosophy extraction (which spec FR-011 explicitly disallows for this slice).
- **Per-section line budgets** (e.g., "Why exists ≤ 12 lines"): partially adopted in FR-001 (each section has a soft target) but not enforced via SC. Reviewer-checked.

---

## R4 — Recording sanitization tooling

**Decision**: Sanitization is enforced at **recording time** via shell-environment manipulation: a clean `PS1`, a placeholder `PWD`, and `env -i` or explicit env scrubbing. No post-processing of the `.cast` JSON is performed.

**Rationale**:
- Per Clarification Q1 (Session 2026-05-06), recordings must mirror spec/017 FR-009: RFC 2606 / RFC 5737, generic prompt, no operator-private values. The simplest enforcement is to run the recording in a controlled shell where these values are already correct.
- Recommended shell setup (documented in `docs/onboarding-script.sh` header):
  ```bash
  env -i HOME=/home/op PATH=/usr/local/bin:/usr/bin:/bin TERM=xterm-256color \
    PS1='op@example $ ' \
    bash --noprofile --norc -c '...recorded commands...'
  ```
- This prevents the operator's real `$HOSTNAME`, `$USER`, `$HOME`, and shell history from appearing in the recording.
- Post-recording, `head -n 1 docs/onboarding.cast | jq` inspects the JSON header. The `env` field of the v2 header includes `TERM` and `SHELL`; both are deterministic when the recording shell is launched as above.
- Verification at PR time: SC-006a runs `grep -i -E '(not\.one|ulthar|192\.168\.|10\.0\.|172\.16\.)' docs/onboarding.cast docs/onboarding-script.sh` and asserts zero matches. Stop-list grep is reliable for the operator's known private markers.

**Alternatives considered**:
- **Post-process the `.cast` JSON with `sed` / `jq`**: rejected. Adds a re-recording step that must run after every regeneration; risks corrupting the JSON; harder to audit than "the recording was clean from the start."
- **Hostname-redaction as a CI lint**: rejected. SC-006a's grep is a stop-list, not a guarantee. CI would need a curated list of all possible operator-private markers, which is a maintenance burden disproportionate to the risk for a docs-only spec. (Consistent with FR-019.)

---

## R5 — Walkthrough block fidelity verification

**Decision**: Verify FR-006 / SC-007a / SC-007b at PR review time by re-running `core-ops plan --source-repo examples/03-immich --host example` against the post-018 master tree and spot-checking each non-elided line in the README walkthrough blocks against the actual command output. **Not a CI gate** (per FR-019).

**Rationale**:
- The walkthrough's purpose is to convey deterministic CLI behavior. The fidelity claim ("every non-elided line is byte-identical to actual output") is verifiable by re-execution.
- A CI-enforced version would require capturing and storing canonical CLI output per master HEAD, then running `diff` against the README blocks. This is feature creep for a docs spec and contradicts FR-019. The reviewer at PR time can run one command and spot-check.
- Drift surface: any future change to `core-ops plan` output formatting (line ordering, color escape codes, header changes) will silently break the README block. The follow-up cost is a manual re-render. This is bounded risk; comparable in cadence to manual asciinema re-recording.

**Alternatives considered**:
- **CI snapshot test**: rejected per FR-019. Would also require a stable example fixture in `tests/` that mirrors `examples/03-immich`, which contradicts FR-017 ("no `tests/` modifications").
- **`include` directive from the README to a separate file**: rejected. Markdown has no native include; would require a build step.
- **Tag-based re-render bot**: out of scope; flagged as a follow-up in synthesis.md if relevant.

---

## Open follow-ups (not blocking 018)

- **F1 — Static SVG sidecar via `agg`**: optional future enhancement to ship `docs/onboarding.svg` (a static SVG-rendered terminal output produced by `agg`) alongside `docs/onboarding.cast`. Provides a no-network static preview for RSS/mirror readers. Not in 018 scope (would expand FR-007).
- **F2 — CONTRIBUTING.md / ARCHITECTURE.md follow-up**: spec.md Assumptions notes that a future spec (019 or beyond) may introduce these based on synthesis findings. Not in 018 scope; flagged in `synthesis.md` if the dogfooding pass surfaces a gap.
- **F3 — Multi-language documentation**: out of scope per spec.md "Out of scope" list. Tracked here for completeness only.
