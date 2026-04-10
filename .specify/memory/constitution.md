<!--
Sync Impact Report:
- Version change: 1.4.0 -> 1.5.0
- Modified principles:
  - 12. Provenance, Versioning, and Behavioral Traceability ->
    12. Provenance, Versioning, and Behavioral Traceability
  - 13. Release Governance and Intent -> 13. Release Governance and Intent
- Modified sections:
  - Core Principles
  - Development Workflow
  - Governance
- Added sections:
  - Principle 13. Release Governance and Intent
- Removed sections: None
- Templates requiring updates:
  - ✅ /home/outergod/code/github.com/outergod/core-ops/.specify/templates/plan-template.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/.specify/templates/spec-template.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/.specify/templates/tasks-template.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/.specify/templates/commands/README.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/AGENTS.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/docs/development.md
  - ✅ /home/outergod/code/github.com/outergod/core-ops/README.md
- Follow-up TODOs: None
-->
# CoreOps Constitution

## Core Principles

### 1. Functional Core, Imperative Shell
Core logic MUST be pure, deterministic, and data-driven wherever feasible.
Side effects such as filesystem mutation, systemd interaction, subprocess
execution, network access, and time-dependent behavior MUST be isolated to thin
boundary layers.

Rationale: Pure reconciliation, diffing, validation, and planning logic is
easier to test, reason about, and regenerate safely.

### 2. Declarative State as the Source of Truth
The system MUST represent desired state, observed state, reconciliation plans,
and outcomes explicitly as data structures. Behavior SHOULD be derived from
transformations over data rather than hidden procedural logic.

Rationale: A GitOps controller must make it obvious what is desired, what
exists, what differs, what will happen next, and what already happened.

### 3. Simplicity Over Cleverness
The project MUST prefer small, composable components, direct control flow, and
minimal abstraction. Abstractions MUST only be introduced when they clearly
reduce complexity or duplication in present use, not hypothetical futures.

Rationale: Long-term maintainability depends more on legibility than on
abstraction density.

### 4. Explicit Effects and Explicit Failure
All side effects, assumptions, and failure modes MUST be visible in interfaces,
types, and return values. Hidden mutation, ambient global state, and silent
recovery MUST be avoided.

Rationale: In a reconciler that mutates host state, implicit behavior is
operational risk.

### 5. Idempotence and Convergence First
Reconciliation MUST be safe to repeat and designed to converge toward declared
state. Reapplying the same desired state MUST NOT cause unintended change.
Retry behavior MUST be deliberate and observable.

Rationale: Idempotence and convergence are foundational properties of any
serious GitOps system.

### 6. Open Standards and Native Interfaces First
The project MUST prefer open standards, documented interfaces, and native
system primitives such as Git, systemd, Quadlet, OCI containers, and standard
Linux facilities. Proprietary dependencies, opaque formats, and unnecessary
custom DSLs SHOULD be avoided.

Rationale: Durability, interoperability, and operator trust depend on staying
close to the platform.

### 7. Observability as a Core Feature
The system MUST make decisions, diffs, plans, applied actions, reconciliation
status, and failures inspectable by operators. Dry-run, auditability, clear
diagnostics, and machine-readable status output SHOULD be treated as core
capabilities, not afterthoughts.

Rationale: Automation without explanation quickly becomes superstition.

### 8. Safe Defaults, Explicit Power
Default behavior MUST minimize surprise and risk. Destructive, disruptive, or
high-impact operations MUST require explicit intent and MUST produce clear
audit output.

Rationale: Safety should be the default posture; sharp tools should still
exist, but only consciously.

### 9. Conservative Public Evolution
Public configuration, state models, file formats, reconciliation status
surfaces, and user-facing behavior SHOULD evolve conservatively. Backward
compatibility SHOULD be preserved where feasible; unavoidable breakage MUST be
explicit, documented, versioned, and justified. Any change that affects
externally observable behavior, persisted state schema, CLI output,
reconciliation semantics, or compatibility MUST evaluate and update the
release version policy accordingly.

Rationale: Mature open source software earns trust by avoiding unnecessary
churn.

### 10. Tests Define the Contract
Tests MUST focus on invariants, externally visible behavior, convergence
guarantees, failure semantics, and provenance-visible outcomes rather than
incidental implementation details. Property-based and scenario-driven tests
SHOULD be preferred where they strengthen confidence.

Rationale: If code is to remain replaceable, the behavioral contract must be
precise and executable.

### 11. Regenerability Over Incidental Craftsmanship
Modules SHOULD be structured so they can be regenerated or rewritten from
specifications and tests without preserving accidental internal structure.
Stable contracts, clear boundaries, and data-oriented design SHOULD be
preferred over clever implementations.

Rationale: AI-assisted development is most effective when the system's truth
lives in specs, interfaces, and tests rather than in opaque internal artistry.

### 12. Provenance, Versioning, and Behavioral Traceability
CoreOps MUST ensure that runtime behavior is traceable to both the reconciler
revision and the desired-state revision actually applied. The system MUST
expose machine-readable provenance and reconciliation status sufficient to
compare revisions, explain behavioral differences, and audit outcomes across
environments. Local controller state MAY be persisted for safety, resumability,
and provenance, but MUST remain derivative rather than authoritative with
respect to desired state. The canonical controller version MUST be the package
version declared in `Cargo.toml`.

Rationale: When behavior diverges across hosts or revisions, operators need
first-class evidence of what code ran, what desired state was applied, and what
outcome resulted.

### 13. Release Governance and Intent
Every releasable change MUST declare explicit release intent and MUST keep trunk
intentionally releasable. A releasable change is any change that affects public
behavior, contracts, release materials, support boundaries, compatibility,
workflow-enforced release behavior, or accepted verification semantics.
Releasable work is incomplete until all of the following are updated together:

- the canonical version in `Cargo.toml`
- `CHANGELOG.md` in Keep a Changelog format
- a machine-checkable release-intent artifact sufficient for CI validation

The effective version bump MUST follow Semantic Versioning and MUST use the
highest applicable bump in the change set: `major` > `minor` > `patch`.
Exemptions MUST be explicit, narrow, and machine-checkable.

Rationale: Automated releases and agent-authored changes are safe only when
release intent is explicit, reviewable, and enforced before merge.

## Additional Constraints

- The nominal tool and project name is `CoreOps`. The Rust crate and binary MAY
  remain `core-ops` for compatibility, packaging, and command-line continuity.

## Development Workflow

Specifications, plans, and tasks MUST document the declarative state model,
side-effect boundaries, idempotence strategy, observability signals,
provenance/version surfaces, compatibility impact, release version policy
impact, changelog impact for externally visible changes, release-intent
artifact impact for releasable work, and the test plan for each change.

Releasable changes MUST update `Cargo.toml`, `CHANGELOG.md`, and the
machine-checkable release-intent artifact in the same change set unless an
explicit exemption rule applies. PRs missing any required release-governance
artifact are invalid and MUST be rejected by CI.

Rust changes MUST pass the project's standard validation gates before a feature
or fix is considered complete. At minimum, this requires `cargo test` and
`cargo clippy --all-targets -- -D warnings`. These gates apply to changes that
modify Rust source, shared test code, or public Rust-facing contracts that can
affect compiled behavior.

Exceptions MUST be explicitly documented in the relevant plan, task, or
implementation record. The exception MUST state why the lint gate is
temporarily waived and what follow-up will restore compliance. Silent waiver of
lint failures is not permitted.

## Governance

- This constitution supersedes all other project guidance.
- Amendments MUST be proposed via documented change, including rationale and
  migration impact, and MUST update the version according to semantic
  versioning.
- Compliance MUST be verified during spec, plan, and task preparation;
  deviations require explicit justification and approval.
- Backward-incompatible governance or principle changes require a MAJOR version
  bump; new principles or material expansions require a MINOR bump;
  clarifications require a PATCH bump.
- Compliance reviews MUST verify that reconciliation status and provenance
  surfaces remain machine-readable, version-comparable, and behaviorally
  explanatory.
- Compliance reviews MUST verify that releasable changes update the SemVer
  impact assessment, continue to derive the canonical controller version from
  `Cargo.toml`, update `CHANGELOG.md`, and include a machine-checkable
  release-intent artifact.
- Compliance reviews MUST reject unspecified exemptions from release-governance
  requirements.
- Compliance reviews MUST verify that Rust changes either pass `cargo test` and
  `cargo clippy --all-targets -- -D warnings` or record an explicit temporary
  exception with remediation.

**Version**: 1.5.0 | **Ratified**: 2026-03-18 | **Last Amended**: 2026-04-10
