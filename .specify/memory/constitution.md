<!--
Sync Impact Report:
- Version change: N/A → 1.0.0
- Modified principles:
  - PRINCIPLE_1_NAME → 1. Functional Core, Imperative Shell
  - PRINCIPLE_2_NAME → 2. Declarative State as the Source of Truth
  - PRINCIPLE_3_NAME → 3. Simplicity Over Cleverness
  - PRINCIPLE_4_NAME → 4. Explicit Effects and Explicit Failure
  - PRINCIPLE_5_NAME → 5. Idempotence and Convergence First
  - PRINCIPLE_5_NAME → 6. Open Standards and Native Interfaces First
  - PRINCIPLE_5_NAME → 7. Observability as a Core Feature
  - PRINCIPLE_5_NAME → 8. Safe Defaults, Explicit Power
  - PRINCIPLE_5_NAME → 9. Conservative Public Evolution
  - PRINCIPLE_5_NAME → 10. Tests Define the Contract
  - PRINCIPLE_5_NAME → 11. Regenerability Over Incidental Craftsmanship
- Added sections: Additional Constraints, Development Workflow, Governance (rules)
- Removed sections: None
- Templates requiring updates: ✅ .specify/templates/plan-template.md; ✅ .specify/templates/spec-template.md; ✅ .specify/templates/tasks-template.md; ⚠ .specify/templates/commands/ (directory missing)
- Follow-up TODOs: None
-->
# core-ops Constitution

## Core Principles

### 1. Functional Core, Imperative Shell
Core logic MUST be pure, deterministic, and data-driven wherever feasible. Side

effects such as filesystem mutation, systemd interaction, subprocess execution,
network access, and time-dependent behavior MUST be isolated to thin boundary
layers.

Rationale: Pure reconciliation, diffing, validation, and planning logic is easier
to test, reason about, and regenerate safely.

### 2. Declarative State as the Source of Truth
The system MUST represent desired state, observed state, reconciliation plans,
and outcomes explicitly as data structures. Behavior SHOULD be derived from
transformations over data rather than hidden procedural logic.

Rationale: A GitOps controller must make it obvious what is desired, what exists,
what differs, what will happen next, and what already happened.

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
state. Reapplying the same desired state MUST NOT cause unintended change. Retry
behavior MUST be deliberate and observable.

Rationale: Idempotence and convergence are foundational properties of any
serious GitOps system.

### 6. Open Standards and Native Interfaces First
The project MUST prefer open standards, documented interfaces, and native system
primitives such as Git, systemd, Quadlet, OCI containers, and standard Linux
facilities. Proprietary dependencies, opaque formats, and unnecessary custom DSLs
SHOULD be avoided.

Rationale: Durability, interoperability, and operator trust depend on staying
close to the platform.

### 7. Observability as a Core Feature
The system MUST make decisions, diffs, plans, applied actions, and failures
inspectable by operators. Dry-run, auditability, and clear diagnostics SHOULD be
treated as core capabilities, not afterthoughts.

Rationale: Automation without explanation quickly becomes superstition.

### 8. Safe Defaults, Explicit Power
Default behavior MUST minimize surprise and risk. Destructive, disruptive, or
high-impact operations MUST require explicit intent and MUST produce clear audit
output.

Rationale: Safety should be the default posture; sharp tools should still exist,
but only consciously.

### 9. Conservative Public Evolution
Public configuration, state models, file formats, and user-facing behavior
SHOULD evolve conservatively. Backward compatibility SHOULD be preserved where
feasible; unavoidable breakage MUST be explicit, documented, and justified.

Rationale: Mature open source software earns trust by avoiding unnecessary churn.

### 10. Tests Define the Contract
Tests MUST focus on invariants, externally visible behavior, convergence
guarantees, and failure semantics rather than incidental implementation details.
Property-based and scenario-driven tests SHOULD be preferred where they
strengthen confidence.

Rationale: If code is to remain replaceable, the behavioral contract must be
precise and executable.

### 11. Regenerability Over Incidental Craftsmanship
Modules SHOULD be structured so they can be regenerated or rewritten from
specifications and tests without preserving accidental internal structure.
Stable contracts, clear boundaries, and data-oriented design SHOULD be preferred
over clever implementations.

Rationale: AI-assisted development is most effective when the system's truth
lives in specs, interfaces, and tests rather than in opaque internal artistry.

## Additional Constraints

No additional constraints are defined beyond the core principles.

## Development Workflow

Specifications, plans, and tasks MUST document the declarative state model,
side-effect boundaries, idempotence strategy, observability signals, compatibility
impact, and the test plan for each change.

## Governance

- This constitution supersedes all other project guidance.
- Amendments MUST be proposed via documented change, including rationale and
  migration impact, and MUST update the version according to semantic versioning.
- Compliance MUST be verified during spec, plan, and task preparation; deviations
  require explicit justification and approval.
- Backward-incompatible governance or principle changes require a MAJOR version
  bump; new principles or material expansions require a MINOR bump; clarifications
  require a PATCH bump.

**Version**: 1.0.0 | **Ratified**: 2026-03-18 | **Last Amended**: 2026-03-18
