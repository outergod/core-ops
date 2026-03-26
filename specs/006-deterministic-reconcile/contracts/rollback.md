# Contract: Rollback Planning and Outcome

## Purpose

Define the minimum externally observable contract for selecting and planning
rollback to a previously successful revision in this iteration.

## Rollback request semantics

A rollback request MUST identify:

- `target_revision_id`
- `scope_id`
- whether the request is `plan_only` or execute-through-normal-reconcile

Rollback MUST be planned through the same three-way reconciliation path used
for forward reconciliation.

## Eligibility rules

A rollback target is eligible only when all of the following are true:

- the target revision was previously recorded as successfully applied
- the retained normalized snapshot for that revision still exists within the rollback window
- the retained snapshot is compatible with the current managed scope
- the planner has enough metadata to construct dependency-aware actions

## Rollback report contract

Rollback planning/reporting MUST identify:

- `target_revision_id`
- `scope_id`
- `eligibility`
- `reason`
- the embedded deterministic plan summary when eligibility succeeds

For the current CLI/report surface, the rollback report is human-readable text
that MUST include:

- `rollback target=<target_revision_id>`
- `eligibility=<eligibility>`
- the standard deterministic plan summary including `scope`, `desired_revision`,
  `baseline_revision`, and ordered action reasoning

Rollback planning MUST fail before execution when eligibility is not
satisfied. Eligibility failure output MUST identify the target revision and
eligibility reason, and it MUST not claim that rollback actions were executed.

## Successful rollback plan contract

A successful rollback plan MUST expose the standard deterministic plan contract
plus rollback-target context. Destructive or disruptive actions MUST remain
visible through the embedded action classifications and reasons.

## Partial rollback result contract

If execution does not fully converge, the deterministic convergence record
persisted for the scope MUST record:

- `desired_revision_id`
- `scope_id`
- `status`
- `attempt_count`
- `affected_objects`
- `completed_actions`
- `failed_actions`
- `can_continue`

## Safety rules

- Rollback MUST NOT be implemented as ad hoc inverse imperative commands.
- Rollback success MUST use the same successful apply boundary as forward reconciliation.
- A failed or partial rollback MUST NOT overwrite `last_applied` for the target revision.
