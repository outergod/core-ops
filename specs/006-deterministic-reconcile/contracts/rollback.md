# Contract: Rollback Planning and Outcome

## Purpose

Define the minimum externally observable contract for selecting and planning rollback to a previously successful revision.

## Rollback request semantics

A rollback request MUST identify:

- `target_revision_id`
- `scope_id`
- whether the request is `plan_only` or execute-through-normal-reconcile

Rollback MUST be planned through the same three-way reconciliation path used for forward reconciliation.

## Eligibility rules

A rollback target is eligible only when all of the following are true:

- the target revision was previously recorded as successfully applied
- the retained normalized snapshot for that revision still exists within the rollback window
- the retained snapshot is compatible with the current managed scope
- the planner has enough metadata to construct dependency-aware actions

## Failure contract

Rollback planning MUST fail before execution when eligibility is not satisfied. Failure output MUST identify:

- `target_revision_id`
- `eligibility_status`
- `eligibility_reason`
- whether execution made any changes (must be `false` for eligibility failure)

## Successful rollback plan contract

A successful rollback plan MUST expose the standard structured diff contract plus:

- `rollback: true`
- `rollback_target_revision_id`
- any destructive or disruptive actions expected during restoration

## Partial rollback result contract

If execution does not fully converge, the result MUST record:

- `rollback_target_revision_id`
- `completed_actions`
- `failed_actions`
- `remaining_drift`
- `can_continue`
- `result_status`

## Safety rules

- Rollback MUST NOT be implemented as ad hoc inverse imperative commands.
- Rollback success MUST use the same successful apply boundary as forward reconciliation.
- A failed or partial rollback MUST NOT overwrite `last_applied` for the target revision.
