# Contract: Provenance Status CLI Surface

## Purpose
Define the operator-facing CLI surface that exposes persisted provenance without creating independent state.

## Rules

- The CLI reads from the canonical persisted provenance status file.
- The CLI must not maintain independent persisted provenance state.
- CLI output may render a human-friendly view and/or a machine-readable view, but both must reflect the canonical file contents.
- When persisted provenance is absent because the file is missing, invalid, partial, or unsupported, the CLI must report absence explicitly rather than fabricating derived state.

## Minimum Information Exposed

- Controller version and revision.
- Desired-state repository and requested ref.
- Last observed revision.
- Last attempted revision.
- Last applied revision.
- Reconciliation generation.
- Reconciliation status, distinguishing at minimum `in_progress`, `success`, and `failed`.
- Explicit never-run state when no reconciliation has ever run.
- Explicit attempted-vs-observed divergence when present.

## Acceptance Expectations

- Two CLI status reads over an unchanged valid status file produce semantically identical output.
- CLI output changes only when the canonical persisted provenance snapshot changes.
- If the canonical status file is invalid or unsupported, the CLI reports provenance as absent rather than showing stale or partial values.
