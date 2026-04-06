# Contract: Verification Run Result Schema

## Purpose

Defines the machine-readable output contract for non-interactive verification
runs and debug/export workflows.

## Schema Shape

```json
{
  "view_kind": "verification_run",
  "run_id": "run-20260401-120001-frontend-idempotency",
  "mode": "ci",
  "controller_version": "0.6.0",
  "revision_selection_basis": "accepted_corpus",
  "revision_under_test": "demo-uat-v2",
  "overall_outcome": "passed",
  "started_at": "2026-04-01T12:00:01Z",
  "completed_at": "2026-04-01T12:07:14Z",
  "scenario_outcomes": [
    {
      "scenario_id": "verify-idempotent-frontend",
      "revision_under_test": "demo-uat-v2",
      "outcome": "passed",
      "failure_summary": null,
      "assertion_results": [
        {
          "assertion_id": "no-pending-change",
          "status": "passed",
          "evidence_refs": ["artifacts/assertions/no-pending-change.json"]
        }
      ]
    }
  ],
  "artifacts": {
    "bundle_path": "artifacts/run-20260401-120001-frontend-idempotency",
    "environment_retained": false
  }
}
```

## Contract Rules

- `view_kind` MUST be `verification_run`.
- `run_id` MUST uniquely identify the run.
- `mode` MUST be `local`, `ci`, or `debug`.
- `controller_version` MUST be sourced from `Cargo.toml`.
- `revision_selection_basis` MUST be `single_scenario` or `accepted_corpus`.
- `revision_under_test` MUST identify the desired-state revision under test.
- `scenario_outcomes[*].revision_under_test` MUST preserve the specific
  desired-state revision exercised by that scenario.
- `overall_outcome` MUST be one of:
  - `passed`
  - `assertion_failure`
  - `infrastructure_failure`
  - `timeout`
  - `harness_error`
- `scenario_outcomes` MUST be present for every scenario attempted in the run.
- `artifacts.bundle_path` MUST reference the retained artifact bundle location.
- `artifacts.environment_retained` MUST indicate whether debug retention kept
  the disposable environment after artifact collection.

## Failure Semantics

- `assertion_failure`: execution completed sufficiently to evaluate one or more
  assertions and at least one authoritative assertion failed.
- `infrastructure_failure`: provisioning, boot, SSH, networking, or other
  environment access failed before the expected behavioral claim could be
  established.
- `timeout`: an explicit readiness, step, or scenario timeout expired.
- `harness_error`: internal harness behavior failed independently of the system
  under test.

## Compatibility Notes

- This output is machine-consumed and stdout in JSON mode from the dedicated
  verification-tool entrypoint MUST remain a single parseable JSON document.
- New fields SHOULD be additive and optional where feasible.
- Changes to meanings, enums, or required fields require release-version review.

## Release-Version-Policy Review Notes

- The `verification_run` JSON payload is a public machine-consumed contract and
  therefore participates in conservative public evolution.
- Adding the dedicated `core-ops-verify` entrypoint and this run-result schema
  as a releasable feature requires a MINOR version increment unless bundled
  into an already-planned MINOR release.
- Additive optional fields may remain compatible, but changes to required
  fields, enum meanings, revision-provenance semantics, or exit-behavior
  interpretation require explicit compatibility review before release.
- The `controller_version` field remains anchored to `Cargo.toml`; contract
  evolution must not introduce a second authoritative controller-version
  source.
