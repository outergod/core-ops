# Quickstart: Deterministic Reconciliation

## Scenario 1: Dry-run a three-way plan

1. Start from a host with an existing successful applied snapshot for a managed scope.
2. Change the desired repository content for one generated unit, one rendered artifact, and one dependent Quadlet resource.
3. Run a dry-run planning command for the new desired revision.
4. Confirm the machine-readable and human-readable plan output both show:
   - ordered actions
   - action classifications
   - dependency context
   - drift categories
   - concise action rationale
5. Repeat the dry-run with unchanged inputs and confirm the ordered plan is materially identical.

## Scenario 2: Detect external drift without desired change

1. Begin with desired state matching the retained successful applied snapshot.
2. Mutate one managed runtime object or rendered artifact out of band.
3. Run planning again.
4. Confirm the plan reports `external_drift`, identifies the affected object, and explains whether CoreOps will correct it automatically.

## Scenario 3: Apply and confirm the successful apply boundary

1. Apply a non-trivial plan containing at least one update and one no-op.
2. Confirm side effects complete.
3. Confirm post-apply verification reports convergence for the managed scope.
4. Confirm the successful applied snapshot now records the new revision as `last_applied`.
5. Confirm a subsequent plan against unchanged host conditions yields no actions.

## Scenario 4: Plan and execute rollback

1. Start from two retained successful revisions for the same managed scope.
2. Select the older retained revision as the rollback target.
3. Run rollback in dry-run mode first and confirm:
   - the plan uses normal reconciliation semantics
   - dependency order is explicit
   - disruptive actions are clearly reported
4. Execute rollback.
5. Confirm the result reports either convergence or partial progress using the same structured outcome model.

## Scenario 5: Surface bounded non-convergence

1. Create a host condition that causes a managed object to fail repeatedly for the same prerequisite reason.
2. Run reconciliation with automatic retry enabled.
3. Confirm CoreOps stops after the bounded retry budget for the same object set and failure pattern.
4. Confirm the result identifies:
   - affected objects
   - repeated failure or oscillation pattern
   - attempts involved
   - whether a later reconcile can continue
