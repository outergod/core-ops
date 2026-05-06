# README Structural Checklist (spec/018)

**Purpose**: Runnable acceptance-criteria runbook for `README.md` after the spec/018 restructure. Used at PR review time and whenever a future contributor proposes a README edit.

**Skeleton.** Populated by T020 in tasks.md Phase 6 (US4) once US1 / US2 / US3 land and the final structure stabilizes. This file currently contains only the table headers and the FR/SC indices each row will reference.

## How to use

1. Run each command from the repository root.
2. Compare the actual output against the expected outcome.
3. Any failure indicates a deviation from the structural contract; resolve before merge.

## Checks (placeholder — populated by T020)

| ID | FR / SC | What | Command | Expected |
|----|---------|------|---------|----------|
| C-001 | FR-003 / SC-001 | README line budget | `wc -l README.md` | ≤ 400 |
| C-002 | FR-002 / SC-002 | Badge row composition | (populated by T020) | 4 badges (CI, E2E Gate, Latest Release, License) |
| C-003 | FR-001 §3 / SC-003 | Mental-model heading position | (populated by T020) | within first 120 lines |
| C-004 | FR-004 / SC-004 | Mermaid block presence | (populated by T020) | ≥ 1 fenced `mermaid` block, contains `Git`, `core-ops`, `systemd` |
| C-005 | FR-006 / SC-007, SC-007b | Walkthrough two-block + budget | (populated by T020) | exactly 2 fenced blocks; combined non-blank ≤ ~25 lines |
| C-006 | FR-009a / SC-006a | Sanitization stop-list | (populated by T020) | 0 matches |
| C-007 | FR-014 / SC-008 | Hype stop-list | (populated by T020) | 0 matches |
| C-008 | Compatibility / SC-009 | Pre-018 link targets resolve | (populated by T020) | every link target exists |
| C-009 | FR-013 | No third-party JS embed | (populated by T020) | 0 matches for `<script>`, `<iframe>`, `asciinema.org/.*\.js` |

See `specs/018-adoption-readiness/spec.md` for the FR/SC definitions this checklist implements.
