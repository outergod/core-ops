# CoreOps Spec — Deterministic Reconciliation

## Status
Draft

## Purpose
Define the principles, semantics, and required functionality for deterministic reconciliation in CoreOps. This iteration introduces a minimal explicit dependency graph and establishes three-way reconciliation, rollback semantics, and convergence handling for single-node operation.

The goal is to ensure that reconciliation is predictable, explainable, reversible, and safe.

---

## 1. Problem Statement

CoreOps currently reconciles desired state into host artifacts and systemd-managed runtime objects, but the system does not yet provide strong guarantees for:

- distinguishing intended drift from local mutation
- reasoning about reconciliation using prior applied state
- rolling back to a known-good revision
- ordering dependent changes safely
- detecting non-converging or oscillating states
- presenting changes in a structured, intelligible form

Without these guarantees, CoreOps risks producing successful-looking reconciliations that are not actually deterministic, reproducible, or debuggable.

---

## 2. Goals

This iteration MUST provide:

1. **Three-way reconciliation** using:
   - desired state
   - last applied state
   - observed actual state
2. **Deterministic ordering** of reconciliation actions using an explicit minimal dependency graph
3. **Revision-based rollback** to previously applied desired state revisions
4. **Convergence detection** including identification of repeated failure or oscillation
5. **Structured diff output** suitable for both humans and agents
6. **Clear reconciliation semantics** for create, update, delete, and no-op decisions
7. **Single-node correctness first** as the basis for later fleet features

---

## 3. Non-Goals

This iteration does NOT attempt to provide:

- multi-node or fleet orchestration
- policy enforcement beyond what is necessary for deterministic reconciliation
- full health-check orchestration beyond convergence-relevant status
- a graphical UI
- distributed rollback across hosts
- fully general dependency modeling beyond CoreOps-managed resources

---

## 4. Design Principles

### 4.1 Determinism over convenience
The same desired revision, applied to the same compatible host conditions, MUST produce the same planned actions and materially equivalent resulting state.

### 4.2 Explicitness over accidental behavior
CoreOps MUST make dependency and reconciliation decisions explicit rather than relying on opaque incidental behavior from systemd or Quadlet alone.

### 4.3 Reversibility over blind forward motion
Every successful apply SHOULD strengthen the system’s ability to return to a previously known applied revision.

### 4.4 Explainability as a first-class requirement
CoreOps MUST be able to answer:

- what changed
- why it changed
- in what order it changed
- what revision caused the change
- why reconciliation failed or was blocked

### 4.5 Mechanical sympathy with systemd
CoreOps MUST not attempt to replace systemd’s runtime execution model. CoreOps owns semantic planning and reconciliation intent; systemd remains the executor of units and dependency mechanics at runtime.

### 4.6 Single-node integrity before distribution
The model introduced here MUST be sufficient to support future fleet management without requiring incompatible redesign.

---

## 5. Terminology

### Desired state
The fully resolved target state after layering and override resolution for a specific reconciliation revision.

### Last applied state
The normalized state snapshot that CoreOps recorded as successfully applied for a specific revision.

### Actual state
The currently observed host state derived from rendered artifacts, systemd unit state, and other CoreOps-managed runtime objects.

### Reconciliation revision
A unique identifier for a specific desired state revision already tracked by CoreOps provenance/revision facilities.

### Plan
The ordered set of reconciliation actions computed from comparison of desired, last applied, and actual state.

### Converged
A state in which actual state materially matches desired state for the managed scope, and no pending reconciliation actions remain.

### Drift
A material difference between actual state and last applied state and/or desired state.

### Rollback target
A previously successful reconciliation revision selected as the desired state to restore.

---

## 6. Scope of Managed Objects

This iteration applies to CoreOps-managed single-node resources including at minimum:

- generated systemd units
- Quadlet-managed service resources
- CoreOps-managed mount and automount resources
- CoreOps-managed rendered host artifacts required for those resources

This iteration MAY include other resource kinds only if they participate in the same normalized state and dependency model.

---

## 7. Core Model

### 7.1 Three-way reconciliation inputs
For each reconciliation run, CoreOps MUST evaluate:

- **desired**: resolved target state for the selected revision
- **last_applied**: normalized snapshot from the most recent successful reconciliation of the same scope
- **actual**: normalized observed state from the host

CoreOps MUST NOT decide actions based solely on desired versus actual when last_applied is available.

### 7.2 Why three-way reconciliation is required
Desired-versus-actual comparison alone cannot distinguish:

- operator-introduced drift
- CoreOps-authored change
- stale residue from a previous desired revision
- out-of-band mutation that should be overwritten, tolerated, or reported

The planner MUST use last_applied to preserve historical continuity in reconciliation reasoning.

### 7.3 Normalized state representation
Desired, last_applied, and actual MUST be normalized into a canonical representation suitable for stable comparison.

Normalization MUST:

- eliminate non-semantic formatting differences
- preserve material semantic differences
- produce stable ordering for deterministic diffing

Normalization rules MUST be documented per resource kind.

---

## 8. Dependency Graph

### 8.1 Requirement
CoreOps MUST construct a minimal explicit dependency graph for all managed objects participating in reconciliation planning.

### 8.2 Purpose
The graph exists to support:

- deterministic action ordering
- cycle detection
- impact analysis
- rollback ordering
- clearer explanations of causality

### 8.3 Node model
At minimum, nodes MAY represent:

- units
- mounts
- automounts
- rendered artifacts
- other managed resources needed for reconciliation semantics

The exact node taxonomy MAY remain implementation-defined, provided semantics remain stable.

### 8.4 Edge model
The graph MUST support:

- explicit dependency edges derived from CoreOps configuration
- implicit dependency edges derived from managed resource semantics

Examples of implicit edges include:

- mount -> unit that consumes the mount
- rendered artifact -> unit generated from or dependent on that artifact

### 8.5 Constraints
CoreOps MUST detect and report dependency cycles before execution.

If a cycle exists in the semantic graph, reconciliation MUST fail before making changes unless the cycle can be reduced into a valid execution order by documented rules.

### 8.6 Relationship to systemd
CoreOps MAY map semantic dependencies onto systemd ordering and requirement directives, but MUST retain its own graph for planning and explanation.

---

## 9. Reconciliation Semantics

### 9.1 Action classes
For each managed object, CoreOps MUST classify reconciliation into one of:

- create
- update
- delete
- replace
- no-op
- blocked

### 9.2 Planning rules
The planner MUST evaluate each object using desired, last_applied, and actual.

At minimum, the planner MUST determine:

- whether the object should exist
- whether the object existed in last_applied
- whether the actual object materially matches desired
- whether actual divergence represents tolerable runtime variance or actionable drift
- whether change to the object requires dependent objects to be updated, restarted, or re-evaluated

### 9.3 Deterministic ordering
Planned actions MUST be executed in a deterministic order derived from the dependency graph.

Creation and update ordering MUST respect dependency prerequisites.
Deletion ordering MUST reverse dependency direction where appropriate.
Rollback ordering MUST also be dependency-aware.

### 9.4 Replace semantics
If an object cannot be safely updated in place, CoreOps MAY classify the action as replace.

Replace semantics MUST define:

- what is removed first
- what is recreated
- what dependent objects are affected
- whether disruption is expected

### 9.5 Blocked semantics
If an action cannot safely proceed because a prerequisite is missing, failed, unresolved, or cyclic, CoreOps MUST mark the action blocked and explain the cause.

---

## 10. Drift Detection

### 10.1 Definition
Drift is any material divergence among desired, last_applied, and actual that affects the semantics of managed state.

### 10.2 Categories
CoreOps SHOULD distinguish at least:

- **expected change**: desired differs from last_applied
- **external drift**: actual differs from last_applied without a corresponding desired change
- **stale residue**: actual reflects an older applied state not matching desired
- **runtime variance**: actual differences that are known and non-semantic

### 10.3 Reporting
Drift output MUST identify:

- affected object
- difference category
- source of truth comparison involved
- whether CoreOps intends to act automatically
- whether user attention is required

### 10.4 Execution semantics
External drift SHOULD be overwritten when desired remains authoritative and no resource-specific exception applies.

Any tolerated drift MUST be explicitly documented rather than silently ignored.

---

## 11. Rollback Semantics

### 11.1 Requirement
CoreOps MUST support rollback to a previously successful reconciliation revision.

### 11.2 Rollback model
Rollback is defined as selecting an earlier successful desired revision as the new desired state and reconciling toward it using the same planner and dependency model.

Rollback MUST NOT be implemented as ad hoc inverse shell actions.

### 11.3 Rollback safety
Rollback planning MUST:

- use three-way reconciliation
- respect dependency ordering
- report destructive or disruptive actions before execution
- fail safely when rollback target is incomplete or incompatible

### 11.4 Rollback target validity
A revision is rollback-eligible only if CoreOps has sufficient normalized state and metadata to plan against it.

### 11.5 Partial failure
If rollback cannot fully converge, CoreOps MUST record:

- target revision
- completed actions
- failed actions
- remaining drift
- whether a subsequent reconcile can continue from current state

---

## 12. Convergence and Non-Convergence

### 12.1 Convergence requirement
After applying a plan, CoreOps MUST evaluate whether managed state converged to desired state.

### 12.2 Non-convergence
CoreOps MUST detect and report at least:

- repeated reconciliation failures on the same object
- oscillation between materially different actual states across attempts
- dependency-blocked states that cannot progress
- replace/restart loops caused by persistent unmet prerequisites or unstable rendering

### 12.3 Retry model
Retry behavior MAY exist, but MUST be bounded and observable.

CoreOps MUST NOT loop indefinitely without surfacing non-convergence.

### 12.4 Non-convergence reporting
When non-convergence is detected, CoreOps MUST identify:

- affected object(s)
- observed pattern
- attempts and recent revisions involved
- whether CoreOps stopped automatically or requires intervention

---

## 13. Structured Diff Output

### 13.1 Requirement
CoreOps MUST produce structured diff output for plans and reconciliation results.

### 13.2 Purposes
Structured diff output exists for:

- operator review
- machine consumption
- later humane interface work
- agent-oriented reasoning
- auditability

### 13.3 Minimum content
Diff output MUST include at least:

- object identity
- action classification
- dependency context
- relevant revision identifiers
- concise explanation of why the action is planned
- material field-level or semantic differences where applicable

### 13.4 Dual form
CoreOps SHOULD produce:

- a machine-readable structured representation
- a concise human-readable rendering derived from the same underlying data

Human rendering MUST NOT be the only source of truth.

---

## 14. Persistence and State Recording

### 14.1 Required persisted data
After successful reconciliation, CoreOps MUST persist enough information to support future three-way reconciliation and rollback.

At minimum this includes:

- reconciliation revision identifier
- normalized last_applied state snapshot
- plan result summary
- dependency metadata or enough data to reconstruct it deterministically

### 14.2 Failure recording
Failed reconciliations SHOULD record enough information to explain incomplete progress and support later diagnosis.

### 14.3 Atomicity expectations
State recording SHOULD avoid falsely marking a revision as successfully applied when execution failed partway.

The implementation MUST define what constitutes a successful apply boundary.

---

## 15. User-Facing Behaviors

### 15.1 Dry-run
CoreOps SHOULD support dry-run planning that computes the full plan, including graph ordering and diffs, without executing changes.

### 15.2 Explainability
A user SHOULD be able to inspect:

- why an object changed
- why an object did not change
- why an action was blocked
- what depends on a changed object
- what revision last changed an object

### 15.3 Failure clarity
Error output MUST prefer causal explanations over raw implementation noise.

---

## 16. Acceptance Criteria

This iteration is complete when all of the following are true:

1. CoreOps computes plans using desired, last_applied, and actual state
2. CoreOps maintains a minimal explicit dependency graph for reconciliation planning
3. Reconciliation order is deterministic and dependency-aware
4. Drift categories are surfaced in plan or status output
5. Rollback to a previous successful revision is supported through the normal reconciliation path
6. Non-converging states are detected and surfaced without infinite retry
7. Structured diff output exists in machine-readable form and is renderable for humans
8. Successful reconciliations persist normalized state sufficient for future three-way reconciliation

---

## 17. Open Questions

The following remain to be specified in follow-up design work unless resolved during this iteration:

1. Which resource fields are considered semantically material per resource kind?
2. Which runtime differences are tolerated as non-semantic variance?
3. What exact persisted schema stores normalized last_applied state?
4. How are restart requirements inferred from specific field changes?
5. What is the minimum rollback history retention policy?
6. What command and output shapes should the humane CLI expose?
7. Which objects become first-class graph nodes versus derived planning details?

---

## 18. Follow-On Work Enabled by This Iteration

This iteration is intended to enable later work on:

- humane operator interface
- fleet management
- policy enforcement
- richer health models
- agent-oriented workflows
- end-to-end reconciliation testing

These later efforts MUST build on the deterministic reconciliation model rather than bypassing it.

