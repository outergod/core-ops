# Quickstart: Provenance and Reconciliation Revision Tracking

## Goal
Inspect the current host provenance and the most recent reconciliation outcome through the canonical persisted status snapshot.

## Canonical Versioning Expectations

- The controller version reported in provenance comes from the package version
  in `Cargo.toml`.
- Changes merged under this feature that alter externally observable behavior
  or persisted-state compatibility must evaluate and update that version
  according to the project versioning policy.
- Backward-incompatible persisted-schema changes require a recorded minor-or-
  major version review outcome before merge.

## What This Iteration Adds

- Controller provenance as identity data.
- Desired-state provenance as observational data.
- Reconciliation provenance as operational state.
- A canonical local status file that is the authoritative persisted provenance source for this iteration.
- Explicit representation of `never_run`, `in_progress`, `success`, and `failed` reconciliation states.

## Expected Operator Flow

1. Run a reconcile attempt through the normal CoreOps workflow.
2. Read the canonical persisted provenance snapshot or run
   `core-ops status --state-file <path>`.
3. Optionally use CLI status output that mirrors the same snapshot.
4. Compare two snapshots to determine whether behavioral differences come from controller identity, desired-state observation, or reconciliation outcome.

## Example Checks

### Successful Reconciliation
- Confirm `status = success`.
- Confirm `last_attempted_revision = last_applied_revision`.
- Confirm `generation` advanced from the previous attempt.

### Failed Reconciliation
- Confirm `status = failed`.
- Confirm `last_attempted_revision` reflects the attempted revision.
- Confirm `last_applied_revision` still reflects the last successful revision.

### In-Progress Reconciliation
- Confirm `status = in_progress` and `running = true`.
- Confirm `last_finished_at` is absent.

### Never-Run State
- Confirm provenance explicitly reports `never_run` rather than relying on missing fields alone.

### Invalid Persisted State
- If the persisted snapshot is partial, invalid, or on an unsupported schema, treat it as absent and investigate why a valid snapshot is unavailable.

## Version Review Record For This Iteration

- Compatibility review outcome: minor version review required for this feature
  because it introduces a new canonical persisted provenance schema and new
  externally observable status behavior.
- Planned controller package version update: `0.1.0 -> 0.2.0`.
