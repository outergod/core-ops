# Feature Specification: Explainable Reconciliation Interface

**Feature Branch**: `007-explainable-reconcile-interface`  
**Created**: 2026-03-27  
**Status**: Draft  
**Input**: User description: "Use `./spec.md` as the input."

## Clarifications

### Session 2026-03-27

- Q: How should the target machine-readable schema replace the currently implemented plan JSON contract? → A: Replace the current plan JSON in place immediately.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Readable Reconciliation Plan (Priority: P1)

As an operator, I want a structured plan view that tells me what CoreOps will change, why each action is needed, and the order in which actions will occur so that I can review reconciliation confidently before execution.

**Why this priority**: A legible plan is the minimum useful surface for explainable reconciliation. Without it, operators cannot predict behavior or validate intent before changes are applied.

**Independent Test**: Can be fully tested by generating plans for representative desired-state revisions and verifying that each changed object is listed once with a stable identifier, deterministic ordering, action classification, dependency context, and an explanation of the cause, while unchanged objects remain discoverable without overwhelming the default view.

**Acceptance Scenarios**:

1. **Given** a desired revision with creates, updates, restarts, deletes, and unchanged objects, **When** an operator requests a plan, **Then** the default plan view emphasizes changed objects first and exposes unchanged objects through a clearly labeled summary or expandable section using the same deterministic ordering and classifications.
2. **Given** a planned action caused by drift or an upstream dependency change, **When** the plan is rendered, **Then** the affected object includes a plain-language explanation of the direct cause and any dependency-driven cause.
3. **Given** the same desired revision and materially identical operating conditions, **When** planning is repeated, **Then** the same objects appear in the same order with materially identical classifications and explanations.

---

### User Story 2 - Live Apply Visibility (Priority: P2)

As an operator, I want live, phase-aware apply output so that I can follow reconciliation progress, understand where the system is spending time, and identify where failures or blockers occurred.

**Why this priority**: Once operators trust the plan, they still need visibility during execution to confirm that apply behavior matches the plan and to react quickly when something goes wrong.

**Independent Test**: Can be fully tested by running apply against representative revisions and verifying that phase transitions, per-object progress states, and final outcomes are streamed in deterministic order and remain consistent with the previously rendered plan.

**Acceptance Scenarios**:

1. **Given** a reconciliation run with multiple dependency-ordered actions, **When** apply starts, **Then** the output visibly enters each reconciliation phase and shows each object moving through pending, running, and terminal result states.
2. **Given** an object that fails during execution, **When** apply reaches that object, **Then** the output reports the failed object, the phase, the cause, and any downstream objects that were blocked or skipped.
3. **Given** a successful apply for a previously rendered plan, **When** the run completes, **Then** the final summary reports the same object identities and ordering seen in the plan together with the terminal convergence result.

---

### User Story 3 - Explainable Results for Automation and Review (Priority: P3)

As an operator or automation agent, I want the same reconciliation information available in both human-readable and machine-readable form so that I can review the output directly or consume it in downstream workflows without semantic drift.

**Why this priority**: Human trust and automation both depend on a single authoritative model. If machine output and human output diverge, neither can be relied on safely.

**Independent Test**: Can be fully tested by comparing human-visible plan/apply/result views with their machine-readable counterparts and verifying that object identity, action meaning, dependency relationships, convergence outcomes, and compatibility-sensitive field contracts match exactly.

**Acceptance Scenarios**:

1. **Given** a reconciliation plan with semantic diffs, dependency explanations, and no-op objects, **When** the system emits machine-readable output, **Then** it contains the same objects, classifications, explanations, result meanings, and ordering semantics as the human-readable view.
2. **Given** a reconciliation run that partially applies and leaves blocked objects, **When** the final result is rendered, **Then** both output forms identify completed work, blocked work, skipped work, and the overall convergence classification.
3. **Given** a request to inspect a single managed object after planning or apply, **When** the object is explained, **Then** the output shows the object identity, its planned or actual action, its dependency context, and the reason for that action or outcome.

### Edge Cases

- What happens when a reconciliation run contains only no-op objects and no material changes are required?
- How does the interface present an object that is unchanged itself but is still affected indirectly by a dependency change?
- What happens when the planner detects a dependency cycle or unresolved prerequisite before execution starts?
- How does the result view distinguish a directly failed object from objects that were blocked or skipped because of that failure?
- What happens when a managed object cannot produce a meaningful line-based diff but still has a material semantic change?
- How does the interface behave when a reconciliation run ends in tolerated variance rather than full convergence?
- How does live presentation remain deterministic if independent objects execute concurrently in a future reconciliation run?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a plan view that accounts for every managed object within the reconciliation scope, including changed and unchanged objects.
- **FR-002**: The plan view MUST present objects in a deterministic order derived from the reconciliation model, especially when explaining dependency trees vs execution order.
- **FR-002a**: The default plan view MUST emphasize changed objects and summarize unchanged or no-op objects separately while preserving access to the full ordered object list.
- **FR-003**: Each object in the plan view MUST use a stable identity composed of a consistent resource type and object name representation.
- **FR-004**: The plan view MUST classify each object as one of: create, update, replace, delete, restart, no-op, blocked, or skipped when applicable to that stage of reconciliation.
- **FR-005**: For every object with a non-no-op classification, the system MUST provide a concise explanation of why that action is required.
- **FR-006**: Explanations MUST distinguish direct causes from dependency-propagated causes when both are present.
- **FR-007**: For every object with a material change, the system MUST provide a normalized, deterministic representation of the semantic difference.
- **FR-007a**: The rendered action classification and explanation MUST be the primary presentation for a changed object, and any diff output MUST appear as supporting evidence rather than replacing the semantic explanation.
- **FR-008**: When a line-oriented comparison is meaningful for an object, the semantic difference MUST be renderable as a unified diff.
- **FR-009**: When a unified diff is not meaningful, the system MUST still provide a deterministic semantic change summary for the object.
- **FR-010**: The system MUST provide a dependency-oriented view that allows operators to inspect prerequisites, dependents, and blockers for a managed object.
- **FR-011**: Dependency views MUST reflect the same ordering and relationship model used by the planner.
- **FR-011a**: The default dependency presentation in plan and apply output MUST be a prerequisite-oriented explanation rooted at each changed object, while dependent and blocker projections remain available through targeted inspection and failure reporting.
- **FR-012**: During apply, the system MUST visibly expose the phases of reconciliation from desired-state resolution through final summary.
- **FR-013**: During apply, each object MUST transition through explicit progress states from pending to running to a terminal result state.
- **FR-014**: Apply output MUST stream object progress using a deterministic presentation model consistent with the corresponding plan, except where blocked or skipped states prevent execution.
- **FR-014a**: Deterministic presentation means each event is attributed to a stable object identity, grouped and summarized in plan order, and rendered so repeated runs with materially identical execution outcomes produce materially identical narration even if independent operations overlap internally.
- **FR-015**: When an object fails, the system MUST report the object identity, the phase of failure, the reported cause, and the downstream impact on related objects.
- **FR-016**: The system MUST distinguish failed, blocked, skipped, and no-op outcomes in both live output and final summaries.
- **FR-017**: The final result MUST explicitly classify the reconciliation outcome as converged, converged with tolerated variance, partially applied, failed, or non-converging.
- **FR-018**: The final result MUST summarize the total number of changed objects, failed objects, blocked objects, skipped objects, and unchanged objects for the run.
- **FR-019**: The same object identities, ordering, and action meanings MUST remain consistent across plan, apply, and final result views for a given reconciliation run.
- **FR-020**: The interface MUST expose revision provenance sufficient for operators to identify the current target revision, the last applied revision, and the revision associated with the reported changes when such context exists.
- **FR-021**: The system MUST support layered summaries that allow operators to consume top-level run status, per-object explanations, and detailed change context from the same underlying reconciliation data.
- **FR-022**: Every human-readable reconciliation view MUST have a machine-readable representation with the same objects, relationships, action meanings, and convergence outcomes.
- **FR-023**: Machine-readable output MUST include plan structure, semantic differences, execution results, dependency relationships, and final convergence classification.
- **FR-024**: Human-readable output MUST be a rendering of the same authoritative reconciliation data used for machine-readable output.
- **FR-024a**: The machine-readable contract MUST treat field names, documented enum values, and documented array ordering semantics as compatibility-sensitive for downstream consumers.
- **FR-024b**: The machine-readable contract MAY add new optional fields in future revisions, but existing documented fields and meanings MUST remain stable within a compatible release line.
- **FR-024c**: When array ordering is used in machine-readable output, that ordering MUST match the documented deterministic presentation semantics and be safe for automated comparison.
- **FR-024d**: The target machine-readable schema defined by this specification MUST replace the currently implemented plan JSON contract in place for this feature, rather than being introduced as a parallel versioned schema.
- **FR-024e**: Implementation work for this feature MUST update existing machine-readable contract tests and consumer-facing documentation to the replacement schema in the same delivery scope so no older plan schema remains normative after release.
- **FR-025**: The system MUST support inspection of an individual managed object after planning or apply, including its identity, action or outcome, dependency context, and explanation.
- **FR-026**: The interface MUST surface a clear outcome for runs that encounter tolerated variance without misreporting them as full convergence.
- **FR-027**: The interface MUST preserve deterministic presentation even when some objects produce no material changes or are excluded from execution because prerequisites failed.
- **FR-028**: The feature MUST apply to the single-node CoreOps reconciliation scope defined by the current managed resources and revision model.
- **FR-029**: Semantic differences MUST align with the normalization rules defined in deterministic reconciliation.
- **FR-030**: Restart explanations MUST identify the triggering change or dependency.
- **FR-031**: The machine-readable output constitutes the authoritative representation of reconciliation state. Human-readable output MUST be a deterministic rendering of this representation.

### Key Entities *(include if feature involves data)*

- **Managed Object**: A single reconciled resource instance shown in plan, apply, diff, dependency, and result views.
- **Plan Entry**: The structured description of a managed object's intended action, order, explanation, and semantic change summary.
- **Execution Event**: A time-ordered record of a managed object's transition through apply phases and terminal outcome.
- **Dependency Relationship**: The declared prerequisite, dependent, or blocking relationship between managed objects used to explain ordering and impact.
- **Convergence Result**: The final classification and summary of a reconciliation run, including its outcome category and affected object counts.
- **Revision Context**: The reconciliation revision information that explains what target state was evaluated, what revision was previously applied, and which revision introduced the current change set.

## Machine-Readable Output Model

The machine-readable output constitutes the **authoritative representation** of reconciliation state for planning, apply progress, result reporting, and object inspection. Human-readable output MUST be a deterministic rendering of this representation and MUST NOT introduce semantics not present in the machine-readable form.

### Design Intent

The machine-readable contract exists to support:

* stable operator tooling
* automated end-to-end testing
* future agentic consumption
* semantic parity between human and machine views

The contract MUST prioritize semantic stability over incidental formatting convenience.

### General Requirements

* All machine-readable output MUST use a documented top-level object shape.
* All objects that represent managed resources MUST use the same stable object identity model.
* Documented field names, enum values, and documented ordering semantics are compatibility-sensitive.
* New optional fields MAY be added in compatible releases.
* Existing documented fields and meanings MUST remain stable within a compatible release line.
* Arrays whose order is semantically meaningful MUST preserve deterministic ordering consistent with reconciliation planning and presentation semantics.
* Fields that are absent MUST have semantics distinct from fields explicitly set to `null`, if both forms are allowed.
* This feature replaces the currently implemented plan JSON contract in place; no parallel versioned machine-readable schema remains normative after release.

## Canonical Entities

### ManagedObjectRef

Identifies a managed object consistently across plan, apply, result, and explain views.

Required fields:

* `resource_type`: stable resource kind identifier
* `name`: stable object name within its kind
* `display_id`: canonical human-oriented identifier derived from `resource_type` and `name`

Example:

```json
{
  "resource_type": "service",
  "name": "app.service",
  "display_id": "service/app.service"
}
```

### RevisionContext

Provides provenance for the reconciliation scope.

Required fields:

* `target_revision`: desired revision being planned or applied

Optional fields:

* `last_applied_revision`: most recent successful applied revision for the same scope
* `change_revision`: revision primarily associated with the current reported change set, when distinguishable

Example:

```json
{
  "target_revision": "abc123",
  "last_applied_revision": "def456",
  "change_revision": "abc123"
}
```

### Cause

Explains why an object has a given action or outcome.

Required fields:

* `kind`: enum describing the cause category
* `summary`: concise human-readable explanation

Optional fields:

* `source_object`: `ManagedObjectRef` when the cause originates from another object
* `details`: structured cause-specific metadata

Allowed `kind` values:

* `desired_change`
* `drift`
* `dependency_change`
* `dependency_failure`
* `blocked_prerequisite`
* `runtime_variance`
* `replacement_required`
* `restart_required`
* `no_change`

Example:

```json
{
  "kind": "dependency_change",
  "summary": "Restart required because container definition changed",
  "source_object": {
    "resource_type": "container",
    "name": "app.container",
    "display_id": "container/app.container"
  }
}
```

### DependencyEdge

Represents a dependency relationship relevant to explanation.

Required fields:

* `relation`: enum
* `object`: `ManagedObjectRef`

Allowed `relation` values:

* `prerequisite`
* `dependent`
* `blocker`

Example:

```json
{
  "relation": "prerequisite",
  "object": {
    "resource_type": "mount",
    "name": "data.mount",
    "display_id": "mount/data.mount"
  }
}
```

### SemanticDiff

Represents the material difference for a changed object.

Required fields:

* `kind`: enum
* `summary`: deterministic semantic summary

Optional fields:

* `unified_diff`: unified diff string when line-based rendering is meaningful
* `details`: structured diff metadata

Allowed `kind` values:

* `line_based`
* `semantic_only`
* `replacement`
* `deletion`
* `creation`

Example:

```json
{
  "kind": "line_based",
  "summary": "Environment variable DB_HOST changed",
  "unified_diff": "--- previous\n+++ desired\n- DB_HOST=old\n+ DB_HOST=new\n"
}
```

## View Shapes

### PlanOutput

Represents the result of reconciliation planning.

Required fields:

* `view_kind`: must be `plan`
* `revision_context`: `RevisionContext`
* `summary`: `PlanSummary`
* `entries`: array of `PlanEntry` in deterministic plan order

#### PlanSummary

Required fields:

* `changed_count`
* `unchanged_count`
* `blocked_count`
* `skipped_count`

Optional fields:

* `total_count`

#### PlanEntry

Required fields:

* `object`: `ManagedObjectRef`
* `action`: enum
* `causes`: non-empty array of `Cause` for non-no-op entries
* `dependencies`: array of `DependencyEdge` containing prerequisite-oriented explanation for the default plan view
* `order_index`: deterministic linearization index

Optional fields:

* `diff`: `SemanticDiff` for materially changed objects
* `unchanged`: boolean convenience field, if retained consistently
* `notes`: array of strings or structured notes

Allowed `action` values:

* `create`
* `update`
* `replace`
* `delete`
* `restart`
* `no_op`
* `blocked`
* `skipped`

Example:

```json
{
  "view_kind": "plan",
  "revision_context": {
    "target_revision": "abc123",
    "last_applied_revision": "def456",
    "change_revision": "abc123"
  },
  "summary": {
    "changed_count": 2,
    "unchanged_count": 1,
    "blocked_count": 0,
    "skipped_count": 0,
    "total_count": 3
  },
  "entries": [
    {
      "object": {
        "resource_type": "mount",
        "name": "data.mount",
        "display_id": "mount/data.mount"
      },
      "action": "no_op",
      "causes": [],
      "dependencies": [],
      "order_index": 0
    },
    {
      "object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      },
      "action": "update",
      "causes": [
        {
          "kind": "desired_change",
          "summary": "Desired environment content differs from last applied state"
        }
      ],
      "dependencies": [],
      "order_index": 1,
      "diff": {
        "kind": "line_based",
        "summary": "Environment variable DB_HOST changed",
        "unified_diff": "--- previous\n+++ desired\n- DB_HOST=old\n+ DB_HOST=new\n"
      }
    },
    {
      "object": {
        "resource_type": "service",
        "name": "app.service",
        "display_id": "service/app.service"
      },
      "action": "restart",
      "causes": [
        {
          "kind": "dependency_change",
          "summary": "Restart required because app.env changed",
          "source_object": {
            "resource_type": "config",
            "name": "app.env",
            "display_id": "config/app.env"
          }
        }
      ],
      "dependencies": [
        {
          "relation": "prerequisite",
          "object": {
            "resource_type": "config",
            "name": "app.env",
            "display_id": "config/app.env"
          }
        },
        {
          "relation": "prerequisite",
          "object": {
            "resource_type": "mount",
            "name": "data.mount",
            "display_id": "mount/data.mount"
          }
        }
      ],
      "order_index": 2
    }
  ]
}
```

### ApplyOutput

Represents streamed or collected execution progress for an apply run.

Required fields:

* `view_kind`: must be `apply`
* `revision_context`: `RevisionContext`
* `phases`: array of `PhaseEvent`
* `events`: array of `ExecutionEvent`
* `summary`: optional until terminal state, required at completion

#### PhaseEvent

Required fields:

* `phase`: enum
* `state`: enum
* `sequence`: deterministic sequence number

Allowed `phase` values:

* `resolution`
* `graph_construction`
* `planning`
* `execution`
* `convergence_check`
* `final_summary`

Allowed `state` values:

* `started`
* `completed`
* `failed`

#### ExecutionEvent

Required fields:

* `object`: `ManagedObjectRef`
* `event_kind`: enum
* `state`: enum
* `sequence`: deterministic narration sequence number

Optional fields:

* `action`: same enum as `PlanEntry.action`
* `cause`: `Cause`
* `phase`: phase enum
* `impacted_objects`: array of `ManagedObjectRef`

Allowed `event_kind` values:

* `object_progress`
* `object_terminal`
* `object_blocked`
* `object_skipped`

Allowed `state` values:

* `pending`
* `running`
* `succeeded`
* `failed`
* `blocked`
* `skipped`

Example:

```json
{
  "view_kind": "apply",
  "revision_context": {
    "target_revision": "abc123",
    "last_applied_revision": "def456"
  },
  "phases": [
    { "phase": "resolution", "state": "started", "sequence": 0 },
    { "phase": "resolution", "state": "completed", "sequence": 1 },
    { "phase": "execution", "state": "started", "sequence": 2 }
  ],
  "events": [
    {
      "object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      },
      "event_kind": "object_progress",
      "state": "running",
      "sequence": 3,
      "action": "update",
      "phase": "execution"
    },
    {
      "object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      },
      "event_kind": "object_terminal",
      "state": "succeeded",
      "sequence": 4,
      "action": "update",
      "phase": "execution"
    }
  ]
}
```

### ResultOutput

Represents the final outcome of a reconciliation run.

Required fields:

* `view_kind`: must be `result`
* `revision_context`: `RevisionContext`
* `outcome`: enum
* `summary`: `ResultSummary`
* `entries`: array of `ResultEntry` in deterministic object order

Allowed `outcome` values:

* `converged`
* `converged_with_tolerated_variance`
* `partially_applied`
* `failed`
* `non_converging`

#### ResultSummary

Required fields:

* `changed_count`
* `failed_count`
* `blocked_count`
* `skipped_count`
* `unchanged_count`

Optional fields:

* `message`

#### ResultEntry

Required fields:

* `object`: `ManagedObjectRef`
* `final_state`: enum

Optional fields:

* `action`: action enum
* `causes`: array of `Cause`
* `dependencies`: array of `DependencyEdge`
* `diff`: `SemanticDiff`

Allowed `final_state` values:

* `succeeded`
* `failed`
* `blocked`
* `skipped`
* `no_op`

Example:

```json
{
  "view_kind": "result",
  "revision_context": {
    "target_revision": "abc123",
    "last_applied_revision": "def456"
  },
  "outcome": "converged",
  "summary": {
    "changed_count": 2,
    "failed_count": 0,
    "blocked_count": 0,
    "skipped_count": 0,
    "unchanged_count": 1,
    "message": "Reconciliation converged successfully"
  },
  "entries": [
    {
      "object": {
        "resource_type": "mount",
        "name": "data.mount",
        "display_id": "mount/data.mount"
      },
      "final_state": "no_op"
    },
    {
      "object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      },
      "final_state": "succeeded",
      "action": "update"
    },
    {
      "object": {
        "resource_type": "service",
        "name": "app.service",
        "display_id": "service/app.service"
      },
      "final_state": "succeeded",
      "action": "restart"
    }
  ]
}
```

### ExplainOutput

Represents targeted inspection of a single managed object.

Required fields:

* `view_kind`: must be `explain`
* `revision_context`: `RevisionContext`
* `object`: `ManagedObjectRef`
* `action_or_outcome`: string or enum, depending on planning vs result context
* `causes`: array of `Cause`
* `dependencies`: array of `DependencyEdge`

Optional fields:

* `diff`: `SemanticDiff`
* `history`: future extension point for revision-local lineage

Example:

```json
{
  "view_kind": "explain",
  "revision_context": {
    "target_revision": "abc123",
    "last_applied_revision": "def456"
  },
  "object": {
    "resource_type": "service",
    "name": "app.service",
    "display_id": "service/app.service"
  },
  "action_or_outcome": "restart",
  "causes": [
    {
      "kind": "dependency_change",
      "summary": "Restart required because app.env changed",
      "source_object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      }
    }
  ],
  "dependencies": [
    {
      "relation": "prerequisite",
      "object": {
        "resource_type": "config",
        "name": "app.env",
        "display_id": "config/app.env"
      }
    }
  ]
}
```

## Human-Readable Rendering Guarantees

Human-readable output MUST be a deterministic rendering of the machine-readable model and MUST preserve:

* object identity
* action meaning
* causal explanation
* dependency context
* plan/apply/result continuity
* convergence classification

Human-readable output SHOULD remain free to evolve in formatting details, provided it continues to satisfy the visibility, ordering, and semantic requirements defined elsewhere in this specification.

Human-readable output MUST NOT require consumers to infer semantics that are absent from the machine-readable contract.

### Plan View Invariants

The default plan view MUST satisfy the following:

#### Ordering and grouping

* Changed objects (create, update, replace, delete, restart) MUST be rendered before unchanged (`no-op`) objects.
* Blocked and skipped objects MUST be rendered in the changed section with their respective classifications.
* Within each section, objects MUST appear in deterministic plan order consistent with the reconciliation model.
* The same ordering MUST be preserved across plan, apply, and result views for the same reconciliation run.

#### Object block structure

Each changed object MUST be rendered as a compact, contiguous block containing, in order:

1. Stable object identity
2. Action classification
3. Concise explanation of the action
4. Dependency context (prerequisite-oriented) when relevant
5. Semantic change evidence (diff or summary) when relevant

The rendering MUST NOT require users to infer any of these elements from surrounding context.

#### Unchanged objects

* Unchanged (`no-op`) objects MUST NOT be fully expanded in the default view.
* The default view MUST include:

  * the total count of unchanged objects
  * a clear mechanism or indication for accessing the full unchanged object list
* When expanded, unchanged objects MUST follow the same deterministic ordering and identity format as changed objects.

#### State distinction

The rendering MUST clearly distinguish between:

* changed (create/update/replace/delete/restart)
* no-op
* blocked
* skipped
* failed (when applicable in plan previews or combined views)

Distinction MUST be visible without requiring diff inspection.

#### Continuity

* The same object identity, action classification, and explanation semantics MUST appear consistently across:
  * plan
  * apply
  * result
* Users MUST be able to correlate a plan entry directly with its execution and final outcome without ambiguity.

### Dependency View Invariants

The dependency-oriented view MUST satisfy the following:

#### Default projection

* The default dependency rendering MUST be prerequisite-oriented and rooted at the selected changed object.
* The root object MUST be clearly identifiable and visually distinct from its dependencies.

#### Identity and consistency

* Each dependency node MUST use the same stable object identity format as the plan view.
* The dependency view MUST reflect the same underlying dependency relationships used by the planner.

#### Hierarchical clarity

* Parent/child relationships MUST be unambiguous through indentation or equivalent hierarchical markers.
* The rendering MUST not require positional inference to understand relationships.

#### Relationship semantics

* Direct prerequisites MUST be distinguishable from transitive prerequisites.
* Blockers, when present, MUST be explicitly labeled as blockers and MUST NOT be rendered as ordinary prerequisites.
* The rendering MUST NOT imply stronger or different relationships than those present in the dependency model.

#### Determinism

* The ordering of dependencies MUST be deterministic across runs with materially identical conditions.
* Repeated renders of the same dependency structure MUST produce materially identical hierarchies.

### Cross-View Invariants

The following apply to all human-readable views:

#### Deterministic rendering

* Rendering MUST be deterministic for materially identical reconciliation inputs and outcomes.
* Incidental differences such as internal concurrency MUST NOT affect visible ordering or grouping.

#### Completeness

* Every visible element in human-readable output MUST correspond to data present in the machine-readable model.
* Human-readable output MUST NOT introduce semantics that are absent from the machine-readable representation.

#### Explanation fidelity

* All explanations shown in human-readable output MUST originate from structured cause data.
* The rendering MUST preserve the distinction between:

  * direct causes
  * dependency-propagated causes

#### Phase awareness

* In apply views, phase transitions MUST be visible and ordered consistently with the reconciliation lifecycle.
* Object progress MUST be attributable to stable object identity at all times.

## Non-Normative Examples

Examples in this specification illustrate typical renderings of plan and dependency views. They are not exact formatting requirements.

Implementations MAY vary in:

* whitespace
* indentation characters
* line wrapping
* stylistic formatting

provided that all rendering invariants and visibility requirements defined above are satisfied.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: This feature specifies operator-facing and agent-facing reconciliation views while keeping comparison, ordering, explanation, and result classification in declarative logic; external execution remains outside the interface contract.
- **Declarative state model**: Plan entries, execution events, dependency relationships, semantic differences, revision context, and convergence results are treated as explicit data that can be rendered consistently in multiple views.
- **Idempotence & convergence**: Repeating plan or apply against materially identical conditions must yield the same visible ordering, identities, explanations, and outcome classification.
- **Explicit effects/failures**: Failures, blockers, skipped work, tolerated variance, and non-converging outcomes are first-class reported states rather than incidental log details.
- **Observability**: The feature makes action intent primary while retaining diffs, phases, dependency context, causal explanations, and convergence summaries as supporting evidence for both interactive operators and automation.
- **Provenance & traceability**: Revision context remains attached to plan, apply, and result views so operators can explain what changed and which revision is responsible.
- **Safe defaults**: The interface emphasizes clear pre-apply planning, explicit failure impact, and unambiguous convergence reporting so operators are not misled into treating partial or blocked runs as successful.
- **Compatibility**: The feature adds an interface contract on top of the existing single-node reconciliation model without changing the managed scope or requiring a graphical interface.
- **Release version policy**: Changes to visible reconciliation semantics, classifications, machine-readable output meaning, or convergence categories must be treated as compatibility-relevant and evaluated in release planning.
- **Test contract**: Acceptance testing must prove stable plan ordering, explanation coverage, phase visibility, failure diagnostics, dependency inspection, plan/apply/result continuity, and human-machine semantic parity.
- **Regenerability**: The specification defines the external behavior of reconciliation views so implementation details can evolve without changing the user-facing contract unintentionally.

## Assumptions

- The feature extends the existing single-node deterministic reconciliation model rather than introducing new reconciliation semantics for multi-node operation.
- Operators need one authoritative model rendered in different views instead of separate plan, apply, and status concepts with unrelated meanings.
- Stable object identity is available for all currently managed CoreOps resources within the reconciliation scope.
- The interface may expose both detailed and summary views, but both must describe the same underlying reconciliation state.
- The default plan output should optimize for readability on non-trivial hosts by foregrounding changed objects and summarizing unchanged objects without hiding them.
- The default dependency explanation should center on why each changed object can or cannot act, with other graph projections available on demand.
- Raw execution logs may appear as supporting detail, but they do not replace structured explanations, result classifications, or semantic diffs.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In acceptance testing with repeated runs under materially identical conditions, plan output preserves object identity, ordering, and action classification in 100% of runs.
- **SC-002**: In 100% of changed objects in acceptance scenarios, structured output includes the object's action classification, direct cause, and dependency context.
- **SC-003**: During acceptance runs with at least five ordered actions, every reconciliation phase transition and every object terminal outcome is visible in the live apply view before the final summary is shown.
- **SC-004**: In 100% of acceptance scenarios involving failure, blockage, skipping, or tolerated variance, the final result correctly distinguishes the run outcome category and names the affected objects.
- **SC-005**: Human-readable and machine-readable outputs match on object identity, action meaning, dependency relationships, and convergence classification in 100% of contract comparison tests.
- **SC-006**: In acceptance scenarios that include unchanged objects, the default plan view surfaces all changed objects without requiring expansion while still exposing the full unchanged object count and access path in 100% of runs.
- **SC-007**: In machine-readable contract tests, documented field names, enum values, and ordering semantics remain unchanged across compatible revisions in 100% of sampled outputs.
- **SC-008**: In 100% of acceptance scenarios, the default human-readable plan view renders all changed objects before unchanged objects while preserving deterministic ordering within each section.
- **SC-009**: In 100% of acceptance scenarios, each changed object in human-readable plan output includes object identity, action classification, and at least one explicit cause derived from structured data.
- **SC-010**: In 100% of dependency-view scenarios, the rendered hierarchy preserves prerequisite ordering, stable object identity, and explicitly labels blockers when present.
- **SC-011**: In repeated runs under materially identical conditions, human-readable plan and dependency views produce materially identical ordering and grouping in 100% of test cases.
