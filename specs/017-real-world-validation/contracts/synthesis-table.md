# Contract: Friction-Classification Synthesis Table

**Phase**: 1 | **Spec**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md) | **Data model**: [../data-model.md#e2--synthesis-table-schema-markdown-rendered-in-specmd](../data-model.md#e2--synthesis-table-schema-markdown-rendered-in-specmd)

The synthesis table is the canonical evidence base for what spec/016 layout decisions held up under real-world translation and what didn't. It is a **markdown table inside `spec.md`**, populated during Phase 2 (Translation) of `/speckit.tasks` and reviewed during Phase 3 (Synthesis). This contract specifies its shape, classification semantics, lifecycle, and invariants.

---

## Location

`specs/017-real-world-validation/spec.md`, in a section titled `## Synthesis table` placed after `## Success Criteria` and before `## Assumptions`. The table is added by a single `/speckit.tasks`-emitted task in Phase 3 (Synthesis); rows are inserted by Phase 2 (Translation) tasks as friction surfaces, then reviewed and finalized in Phase 3.

If translation surfaces zero friction (the spec/016 layout was sufficient as-shipped for all five setups), the section reads:

```markdown
## Synthesis table

No friction surfaced during translation of the five real-world setups. The
spec/016 source-repository layout was sufficient as-shipped. SC-002 is
trivially satisfied.
```

---

## Row shape

Each row is exactly one friction. Columns and column order:

| # | Column | Allowed values | Required? |
|---|--------|----------------|-----------|
| 1 | **Friction** | One-line prose; ≤ 100 chars; describes the layout gap or CLI gap encountered. | Yes |
| 2 | **Affected examples** | Comma-separated example slugs from {`01-caddy-whoami`, `02-nextcloud`, `03-immich`, `04-traefik-authelia`, `05-observability`}. | Yes (≥ 1 slug) |
| 3 | **Classification** | Exactly one of `A`, `B`, `C` (case-sensitive). | Yes |
| 4 | **Rationale** | One-line prose; ≤ 200 chars; explains why this classification fits. | Yes |
| 5 | **Action** | One of: `Escalate to spec/<NNN>`, `Documented in <slug>/README.md`, `Tracked in docs/follow-ups.md`. | Yes |

Markdown rendering:

```markdown
| Friction | Affected examples | Classification | Rationale | Action |
|----------|-------------------|----------------|-----------|--------|
| Stateless plan against examples blocked all five fixtures | 01..05 | A | Layout was fine; the bottleneck was a missing CLI flag (`--source-repo`) shared across the entire roster | Escalate to spec/017 (this iteration absorbs the fix) |
| <…another row…> | <…> | <…> | <…> | <…> |
```

---

## Classification semantics

### `A` — Amend-now (escalate to follow-up spec)

- **Trigger**: ≥ 2 of 5 examples are blocked by *the same* layout gap (i.e., the spec/016 layout cannot express something necessary across multiple real workloads).
- **Required action**: row's Action MUST read `Escalate to spec/<NNN>` where `<NNN>` is a real (or to-be-created-imminently) follow-up spec number.
- **Implication**: spec/017 itself does NOT land the layout amendment. The escalation creates a separate spec branch and PR. spec/017 ships with the friction documented and the follow-up referenced.
- **Exception**: if the gap is CLI-level rather than layout-level and the fix is small, this slice MAY absorb the fix inline. In that case the Action is `Escalate to spec/017 (this iteration absorbs the fix)` (per the 2026-05-05 stateless-mode example). Self-escalation requires explicit operator approval recorded in spec.md Clarifications.

### `B` — Workaround-with-doc (default)

- **Trigger**: friction is real and addressable via a reserved-prefix subdir, host-side preparation step, drop-in trick, or other documented pattern. The workaround does not require any layout or CLI change.
- **Required action**: row's Action MUST read `Documented in <slug>/README.md` where `<slug>` is one of the affected example slugs. The example's README MUST contain a `## Known limitations` heading with the friction's name and the workaround text.
- **Implication**: future authors who hit this friction find the workaround in the example README and the row in this synthesis table.

### `C` — Defer-to-spec-018 (acknowledged but not addressed)

- **Trigger**: friction is real, neither blocking nor addressable in this slice. The translator could not find a clean workaround AND the gap does not block ≥ 2 examples (so it doesn't qualify for A).
- **Required action**: row's Action MUST read `Tracked in docs/follow-ups.md`. The corresponding bullet MUST exist in `docs/follow-ups.md` by the time spec/017 merges.
- **Implication**: spec/017 ships with the friction surfaced in the synthesis table and the follow-up entry. A future spec (e.g., spec/018) decides whether to address it.

---

## Invariants

The synthesis table MUST satisfy these invariants by the time the slice merges. They are review-time invariants, not machine-checked, but `/speckit.analyze` (the spec-kit analysis command, if run) SHOULD flag violations.

1. **Coverage with example READMEs**: every friction surfaced in any `examples/<slug>/README.md` (under `## Known limitations` or equivalent) MUST appear as a row in this table. (SC-002.)
2. **Classification A self-consistency**: any row classified `A` whose Action is `Escalate to spec/<NNN>` MUST reference an `<NNN>` that either already exists or will exist within one merge cycle of spec/017. Dangling references are spec drift.
3. **Classification B self-consistency**: any row classified `B` MUST have its workaround text in at least one of the affected examples' README under `## Known limitations`.
4. **Classification C self-consistency**: any row classified `C` MUST correspond to a bullet in `docs/follow-ups.md` whose text references the friction.
5. **No `A`-classified rows whose action is `Escalate to spec/017 (absorbed)` exist beyond the stateless-mode case**: any other in-scope absorption is a scope creep that should have been declared during `/speckit.clarify` and not at merge time.

---

## Lifecycle

```text
   /speckit.tasks (Phase 3 task) inserts an empty
              ## Synthesis table section.
                       │
                       ▼
   Translation tasks (Phase 2) populate rows as friction surfaces:
       - Each translator writes a row in the table at the time
         friction is encountered, marking classification A/B/C
         per the operator's judgment.
       - Each translator updates the affected example's
         README "Known limitations" section (B classification)
         OR opens a follow-up bullet in docs/follow-ups.md
         (C classification) OR opens a follow-up spec
         placeholder (A classification) — same commit.
                       │
                       ▼
   Synthesis task (Phase 3) reviews the populated table:
       - Check invariants 1-5 above.
       - Promote any C rows to A if a second example surfaces
         the same friction (the threshold for amend-now is hit).
       - Demote any A rows to C if the "≥ 2 examples" condition
         did not actually hold.
       - Final pass: row count, action consistency, dangling refs.
                       │
                       ▼
   Slice merge: synthesis table becomes the evidence record.
```

---

## Future evolution

- If/when the table grows beyond ~20 rows (a sign that the spec/016 layout has substantial friction across many workloads), promote it from a markdown table in spec.md to a structured YAML or TOML data file with a separate validator binary. Out of scope for this slice.
- The classification system itself (A/B/C) is intentionally minimal. Future iterations may add classifications (e.g., `D` for "duplicate of an A elsewhere"), but the additive change requires a spec amendment.
