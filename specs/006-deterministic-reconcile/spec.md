# Feature Specification: Deterministic Reconciliation

**Feature Branch**: `006-deterministic-reconcile`  
**Created**: 2026-03-25  
**Status**: Draft  
**Input**: User description: "Define the principles, semantics, and required functionality for deterministic reconciliation in CoreOps, including three-way reconciliation, explicit dependency ordering, rollback semantics, convergence handling, and structured diff output for single-node operation."

## Clarifications

### Session 2026-03-25

- Q: What constitutes a successful apply boundary for persisted last_applied state? → A: Success is reached only after planned side effects complete and post-apply verification confirms convergence for the managed scope.
- Q: Which revisions are rollback-eligible? → A: Only previously successful revisions retained within a defined rollback history window are rollback-eligible.
- Q: When should automatic retry stop for non-converging reconciliation? → A: Automatic retry stops after a small fixed retry budget is exhausted for the same affected object set and failure pattern.

## Non-Goals

- Multi-node or fleet orchestration
- Policy enforcement beyond what is necessary for deterministic reconciliation
- Full health-check orchestration beyond convergence-relevant status
- A graphical user interface
- Distributed rollback across hosts
- Fully general dependency modeling beyond CoreOps-managed resources

## Design Principles

### Determinism over convenience

The same desired revision, applied to the same compatible host conditions, MUST produce the same planned actions and materially equivalent resulting state.

### Explicitness over accidental behavior

CoreOps MUST make dependency and reconciliation decisions explicit rather than relying on opaque incidental behavior from systemd or Quadlet alone.

### Reversibility over blind forward motion

Every successful apply SHOULD strengthen the system's ability to return to a previously known applied revision.

### Explainability as a first-class requirement

CoreOps MUST be able to answer:

- What changed
- Why it changed
- In what order it changed
- What revision caused the change
- Why reconciliation failed or was blocked

### Mechanical sympathy with systemd

CoreOps MUST not attempt to replace systemd's runtime execution model. CoreOps owns semantic planning and reconciliation intent; systemd remains the executor of units and dependency mechanics at runtime.

### Single-node integrity before distribution

The model introduced here MUST be sufficient to support future fleet management without requiring incompatible redesign.

## Terminology

- **Desired state**: The fully resolved target state after layering and override resolution for a specific reconciliation revision.
- **Last applied state**: The normalized state snapshot that CoreOps recorded as successfully applied for a specific revision.
- **Actual state**: The currently observed host state derived from rendered artifacts, systemd unit state, and other CoreOps-managed runtime objects.
- **Reconciliation revision**: A unique identifier for a specific desired state revision already tracked by CoreOps provenance and revision facilities.
- **Plan**: The ordered set of reconciliation actions computed from comparison of desired, last applied, and actual state.
- **Converged**: A state in which actual state materially matches desired state for the managed scope and no pending reconciliation actions remain.
- **Drift**: A material difference between actual state and last applied state and/or desired state.
- **Rollback target**: A previously successful reconciliation revision selected as the desired state to restore.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Explainable Three-Way Planning (Priority: P1)

As an operator, I want CoreOps to compute reconciliation decisions from desired state, last applied state, and actual state so that I can understand what changed, why it changed, and whether the system is correcting expected change or external drift.

**Why this priority**: This is the core behavior change for the iteration. Without three-way planning, rollback, drift reasoning, and deterministic reconciliation are not trustworthy.

**Independent Test**: Can be fully tested by providing the same desired revision with controlled last applied and actual states, then verifying that CoreOps produces the same structured plan, classifications, and explanations on repeated runs.

**Acceptance Scenarios**:

1. **Given** a desired state revision, a recorded successful applied snapshot, and an actual host state with external drift, **When** CoreOps computes a plan, **Then** the plan identifies the affected managed objects, classifies the drift, and explains why corrective actions are required.
2. **Given** a desired state revision, a matching last applied snapshot, and an actual host state that materially matches desired state, **When** CoreOps computes a plan, **Then** the plan reports no-op decisions and no unnecessary actions.
3. **Given** identical desired, last applied, and actual inputs on the same compatible host conditions, **When** planning is repeated, **Then** CoreOps produces materially identical action ordering, classifications, and diff output.

---

### User Story 2 - Safe Revision Rollback (Priority: P2)

As an operator, I want to select a previously successful revision and reconcile back to it using the normal planner so that rollback is predictable, dependency-aware, and auditable.

**Why this priority**: Rollback is a major safety mechanism, but it depends on the correctness of three-way planning and persisted applied state.

**Independent Test**: Can be fully tested by reconciling to one successful revision, moving to a later revision, then selecting the earlier successful revision and verifying that CoreOps computes and reports a dependency-aware rollback plan using the same reconciliation path.

**Acceptance Scenarios**:

1. **Given** a previously successful revision with sufficient applied-state metadata, **When** an operator selects it as the desired target, **Then** CoreOps plans rollback through the same reconciliation model instead of ad hoc inverse actions.
2. **Given** a rollback target that lacks sufficient normalized state or is incompatible with current managed scope, **When** rollback planning is requested, **Then** CoreOps fails safely before execution and explains why rollback cannot proceed.
3. **Given** a rollback that only partially succeeds, **When** execution stops, **Then** CoreOps records completed actions, failed actions, remaining drift, and whether a later reconcile can continue.

---

### User Story 3 - Non-Convergence Detection and Structured Reporting (Priority: P3)

As an operator or automation agent, I want CoreOps to detect blocked, repeated, or oscillating reconciliation behavior and emit structured output so that failures are diagnosable and safe to automate around.

**Why this priority**: Deterministic reconciliation is incomplete if the system cannot identify when convergence is failing or why retries should stop.

**Independent Test**: Can be fully tested by simulating repeated failure, dependency blockage, and oscillating actual state, then verifying that CoreOps emits structured results that identify the affected objects, failure pattern, and stop conditions without infinite retry.

**Acceptance Scenarios**:

1. **Given** a managed object that repeatedly fails for the same prerequisite reason, **When** CoreOps performs bounded reconciliation attempts, **Then** it reports non-convergence with the affected object, attempts involved, and the blocking cause.
2. **Given** actual state that alternates between materially different values across attempts, **When** CoreOps detects the pattern, **Then** it reports oscillation rather than retrying indefinitely.
3. **Given** a plan with creates, updates, deletes, replacements, no-ops, and blocked actions, **When** CoreOps renders reconciliation output, **Then** both machine-readable and human-readable representations describe the same underlying structured result.

### Edge Cases

- What happens when last applied state is missing for a managed scope but actual state contains residual artifacts from older revisions?
- How does the system handle a dependency cycle discovered in semantic planning before any side effects occur?
- What happens when rollback is requested to a revision whose normalized snapshot exists but no longer covers the current managed object set?
- How does the system behave when actual state contains tolerated runtime variance mixed with actionable external drift on the same object?
- What happens when a replacement action is required for an object with dependents that cannot be safely restarted in the same run?
- How does the system report a partially executed reconciliation when some actions succeeded before a later action became blocked?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CoreOps MUST compute reconciliation decisions from three inputs for the same managed scope: desired state, last applied state, and actual observed state.
- **FR-002**: CoreOps MUST normalize desired, last applied, and actual state into canonical representations suitable for deterministic comparison.
- **FR-003**: Normalization MUST eliminate non-semantic formatting differences while preserving material semantic differences relevant to reconciliation.
- **FR-004**: CoreOps MUST document normalization rules for each managed resource kind participating in this iteration.
- **FR-005**: CoreOps MUST NOT decide reconciliation solely from desired-versus-actual comparison when last applied state is available for the same scope.
- **FR-006**: CoreOps MUST construct an explicit minimal dependency graph for managed objects participating in reconciliation planning.
- **FR-007**: The dependency graph MUST support deterministic ordering, cycle detection, rollback ordering, and causal explanation of planned actions.
- **FR-008**: CoreOps MUST include both explicit dependencies from CoreOps-managed configuration and implicit dependencies derived from managed resource semantics.
- **FR-009**: CoreOps MUST detect semantic dependency cycles before execution and fail safely unless a documented reduction rule yields a valid execution order.
- **FR-010**: CoreOps MUST classify each managed object decision as create, update, delete, replace, no-op, or blocked.
- **FR-011**: For each managed object, CoreOps MUST determine whether the object should exist, whether it existed in last applied state, whether actual materially matches desired, and whether dependent objects require re-evaluation.
- **FR-012**: Planned execution order for create, update, replace, and delete actions MUST be deterministic and dependency-aware.
- **FR-013**: Delete and rollback ordering MUST reverse dependency direction where required for safe execution.
- **FR-014**: Replace semantics MUST identify what is removed, what is recreated, which dependents are affected, and whether disruption is expected.
- **FR-015**: If an action cannot safely proceed due to unresolved prerequisites, dependency blockage, or cycles, CoreOps MUST mark it blocked and explain the cause.
- **FR-016**: CoreOps MUST distinguish at least expected change, external drift, stale residue, and runtime variance in reconciliation reporting.
- **FR-017**: Drift output MUST identify the affected object, the drift category, the comparison basis, whether CoreOps will act automatically, and whether user attention is required.
- **FR-018**: Any tolerated drift or runtime variance MUST be explicitly documented and surfaced rather than silently ignored.
- **FR-019**: CoreOps MUST support rollback by selecting a previously successful revision as the new desired target and reconciling toward it through the normal planner.
- **FR-020**: Rollback MUST use the same three-way reconciliation model, dependency graph, and action classifications as forward reconciliation.
- **FR-021**: CoreOps MUST reject rollback targets that lack sufficient normalized state or required metadata to plan safely.
- **FR-021a**: CoreOps MUST define and enforce a bounded rollback history retention window; only previously successful revisions still retained within that window are rollback-eligible.
- **FR-022**: When rollback or forward reconciliation cannot fully converge, CoreOps MUST record completed actions, failed actions, remaining drift, and whether later reconciliation can continue.
- **FR-023**: After each apply attempt, CoreOps MUST evaluate whether managed state converged to desired state for the managed scope.
- **FR-024**: CoreOps MUST detect and report repeated failure, oscillation, dependency-blocked states, and bounded retry exhaustion.
- **FR-025**: CoreOps MUST NOT retry indefinitely without surfacing non-convergence and stopping automatically or requiring intervention.
- **FR-025a**: CoreOps MUST use a small fixed retry budget for repeated failure or oscillation affecting the same object set and failure pattern, then stop automatic retry and require operator intervention.
- **FR-026**: CoreOps MUST produce structured diff output for plans and reconciliation results in a machine-readable representation.
- **FR-027**: CoreOps MUST provide a human-readable rendering derived from the same structured diff representation used for machine output.
- **FR-028**: Structured diff output MUST include object identity, action classification, dependency context, relevant revision identifiers, concise action rationale, and material semantic differences.
- **FR-029**: After each successful reconciliation, CoreOps MUST persist enough normalized state and metadata to support future three-way reconciliation and rollback for the same scope.
- **FR-030**: Persisted reconciliation state MUST include the applied revision identifier and enough plan or dependency data to reconstruct later reasoning deterministically.
- **FR-031**: Failed reconciliations SHOULD persist enough progress and diagnostic context to explain incomplete execution without falsely marking the revision as successfully applied.
- **FR-031a**: CoreOps MUST treat a reconciliation revision as successfully applied only after planned side effects complete and post-apply verification confirms convergence for the managed scope.
- **FR-032**: CoreOps SHOULD support dry-run planning that computes graph ordering, action classifications, dependency context, and structured diffs without executing side effects.
- **FR-033**: CoreOps MUST be able to answer, for a reconciliation run, what changed, why it changed, in what order it changed, which revision caused the change, and why reconciliation failed or was blocked.
- **FR-034**: This iteration MUST apply at minimum to CoreOps-managed generated systemd units, Quadlet-managed service resources, CoreOps-managed mount and automount resources, and CoreOps-managed rendered host artifacts required by those resources.
- **FR-035**: CoreOps MUST preserve single-node correctness and deterministic behavior as the primary goal of this iteration, with semantics compatible with later fleet-oriented extension.

### Key Entities *(include if feature involves data)*

- **Desired State Revision**: The fully resolved target state for a specific reconciliation revision after layering and override resolution.
- **Applied State Snapshot**: The normalized state that CoreOps previously recorded as successfully applied for a specific revision and managed scope.
- **Observed State Snapshot**: The normalized description of actual host state for managed objects at reconciliation time.
- **Managed Object**: A CoreOps-managed resource instance participating in planning, diffing, execution, rollback, or convergence evaluation.
- **Dependency Graph**: The minimal explicit semantic graph used to order actions, detect cycles, explain causality, and support rollback.
- **Reconciliation Plan**: The ordered set of create, update, delete, replace, no-op, and blocked decisions produced from desired, last applied, and actual state.
- **Drift Record**: A structured description of a material divergence, including its category, affected object, comparison basis, and intended CoreOps response.
- **Rollback Target**: A previously successful desired revision selected for restoration through the normal reconciliation path.
- **Convergence Record**: The structured outcome of reconciliation attempts, including success, blockage, repeated failure, oscillation, and remaining drift.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Three-way comparison, normalization, dependency graph construction, action classification, drift categorization, rollback planning, and convergence detection remain declarative planning logic; filesystem, systemd, and runtime mutation stay in imperative boundary layers.
- **Declarative state model**: This feature formalizes desired state, last applied state, actual observed state, dependency graph, reconciliation plan, drift records, and convergence outcomes as explicit data structures.
- **Idempotence & convergence**: Reapplying the same desired revision against materially equivalent host conditions must yield the same plan and converged result; retries are bounded and non-convergence is surfaced explicitly.
- **Explicit effects/failures**: Blocked actions, dependency cycles, rollback ineligibility, oscillation, and partial progress are first-class outcomes, not implicit side effects.
- **Observability**: Structured diffs, causal action explanations, dependency context, dry-run planning, convergence diagnostics, and rollback reporting are core outputs of the feature.
- **Provenance & traceability**: Reconciliation revisions, last applied state, rollback targets, and convergence outcomes are persisted so operators can compare revisions and explain behavior.
- **Safe defaults**: Rollback and replacement remain planner-mediated operations with explicit dependency-aware reporting before execution; failed or partial runs must not be marked successful.
- **Compatibility**: The model is designed to extend existing CoreOps-managed resources without replacing systemd runtime execution; future fleet work must build on this semantic model without incompatible redesign.
- **Release version policy**: This feature changes reconciliation semantics, persisted reconciliation state expectations, rollback behavior, and operator-visible diff/status output, so release version impact must be evaluated explicitly during planning.
- **Test contract**: Tests must prove deterministic planning, cycle detection, drift categorization, rollback planning, bounded retry behavior, non-convergence signaling, and machine-readable diff output.
- **Regenerability**: Specs, state models, and tests define the semantic contract so planning and reconciliation logic can be regenerated without relying on incidental implementation structure.

## Assumptions

- Single-node reconciliation scope is sufficient for this iteration; multi-node coordination is intentionally deferred.
- CoreOps already has revision and provenance facilities that can identify desired revisions and previously successful reconciliations.
- Resource-specific definitions of semantically material fields and tolerated runtime variance may start with the managed resource kinds already supported by CoreOps and can be expanded later without changing the core three-way model.
- Operators prefer explicit blocked or failed outcomes over silent tolerance of ambiguous drift.
- Rollback is initiated by selecting a previously successful revision rather than by issuing inverse imperative commands.
- A revision is not eligible to become `last_applied` until post-apply verification confirms convergence for the managed scope.
- Rollback history is intentionally bounded; once the retained normalized snapshot for a previously successful revision expires or is discarded, that revision is no longer rollback-eligible.
- Automatic retry is intentionally bounded by a small fixed budget for the same affected object set and failure pattern rather than by resource-specific policies in this iteration.

## Open Questions

- How are restart requirements inferred from specific field changes?
- Which objects become first-class graph nodes versus derived planning details?

## Follow-Up Questions

- What default rollback retention window should later product policy adopt beyond the bounded-history requirement established in this iteration?
- What humane CLI presentation shapes should later operator-focused UX layers expose on top of the core machine-readable and human-readable reporting introduced here?

## Deferred Design Notes

- This iteration MUST establish the framework and persistence hooks for per-resource normalization and tolerated runtime variance, but exhaustive per-resource rule tables may be finalized during implementation and captured in contracts and operator documentation before feature completion.
- This iteration MUST define a persisted normalized-state schema sufficient for three-way reconciliation, rollback eligibility, and convergence reporting, but the final field-level schema shape may be refined during implementation and recorded in the machine-readable contracts before release validation.

## Follow-On Work Enabled by This Iteration

- Humane operator interface
- Fleet management
- Policy enforcement
- Richer health models
- Agent-oriented workflows
- End-to-end reconciliation testing

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In controlled repeat trials with identical desired, last applied, and actual inputs, CoreOps produces the same ordered plan and materially equivalent structured diff output in 100% of runs.
- **SC-002**: Operators can identify the action classification, dependency context, revision cause, and concise explanation for every planned change from the structured plan output without inspecting implementation internals.
- **SC-003**: Rollback to an eligible previously successful revision can be planned and executed through the normal reconciliation path in under 5 minutes for a representative single-node managed scope used in acceptance testing.
- **SC-004**: In acceptance scenarios covering repeated failure, dependency blockage, and oscillation, CoreOps stops bounded retries and surfaces the non-convergence pattern, affected objects, and recent attempt context in 100% of cases.
- **SC-005**: After a successful reconciliation, subsequent planning against unchanged compatible host conditions produces no action plan and reports converged state for the managed scope in 100% of acceptance runs.
