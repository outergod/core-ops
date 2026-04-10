# Research: SemVer and Changelog Governance

## Decision: Use checked-in release fragments as the single machine-checkable release-intent mechanism

### Rationale

Checked-in release fragments keep release intent in the reviewed branch state,
make the declaration auditable in Git history, and avoid relying on mutable PR
labels or out-of-band metadata. They also let CI validate the exact same
artifact that maintainers review.

### Alternatives considered

- Shared manifest file: rejected because it creates central merge pressure and
  increases conflict risk under trunk-based development.
- PR labels or PR metadata: rejected because they are weaker as repository
  truth and are less durable in Git history.
- Direct changelog edits only: rejected because they are not sufficiently
  machine-checkable for deterministic SemVer validation.

## Decision: Expose governance validation through a sibling helper binary rather than the main `core-ops` CLI

### Rationale

Release governance is maintainer and CI tooling, not part of the normal
operator-facing reconciliation surface. A dedicated helper binary keeps
`core-ops` focused on runtime behavior while still providing a stable,
testable, repo-local command for contributors, agents, and workflows.

### Alternatives considered

- Main `core-ops` subcommand: rejected because it expands the operator-facing
  CLI with repository-maintenance behavior.
- Workflow-only shell logic: rejected because it would duplicate policy outside
  the Rust codebase and weaken testability.

## Decision: Generate `CHANGELOG.md` from approved release fragments

### Rationale

Generating `CHANGELOG.md` from fragments removes per-PR contention on a shared
document while preserving a human-readable published changelog. It aligns well
with trunk-based development and avoids requiring every PR to own a final
release heading manually.

### Alternatives considered

- Direct changelog editing in every releasable PR: rejected because it creates
  conflict pressure and implies a rigid one-PR-one-heading model.
- No persistent changelog file: rejected because CoreOps already treats
  `CHANGELOG.md` as part of the public distribution surface.

## Decision: Encode release classification with policy tables, not only examples

### Rationale

The feature depends on deterministic classification of releasable versus exempt
changes and deterministic SemVer impact assignment. A policy table anchored in
the spec gives maintainers and agents the same decision model and reduces
inconsistent classification.

### Alternatives considered

- Relying on examples only: rejected because it leaves too much semantic room
  for different implementations.
- Purely path-based classification: rejected because some cases, especially
  workflows, tests, fixtures, and examples, require semantic evaluation.

## Decision: Reject metadata-only PRs unless explicitly marked as release-preparation work

### Rationale

Leaving metadata-only PR behavior open would turn governance into convention
instead of contract. Requiring an explicit release-preparation designation keeps
normal PRs honest while still allowing intentional release preparation when
needed.

### Alternatives considered

- Always allow metadata-only changes: rejected because it weakens the meaning
  of release metadata and invites noisy or speculative version churn.
- Always reject metadata-only changes: rejected because intentional release
  preparation can be a valid workflow.

## Decision: Treat accepted verification corpus changes as at least patch-level releasable changes

### Rationale

Accepted verification corpus entries are part of the project’s asserted
behavioral contract and release credibility surface. Changing them affects what
CoreOps claims to verify, so such changes require version traceability even
without Rust source modifications.

### Alternatives considered

- Exempt accepted scenario changes: rejected because it would let externally
  meaningful claims shift without release visibility.
- Classify only release-gate scenario changes as releasable: rejected because
  the accepted corpus is a broader behavioral contract than the gate alone.
