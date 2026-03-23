# Contract: Canonical Persisted Provenance Status File

## Purpose
Define the canonical machine-readable file that stores persisted provenance for a host in this iteration.

## Scope
- Authoritative persisted provenance source for this iteration.
- Represents current state and last reconciliation outcome only.
- Does not represent historical sequence analysis or a reconciliation journal.

## Required Top-Level Structure

```json
{
  "schema_version": 1,
  "controller": {
    "version": "0.1.0",
    "revision": "8f3c2ab",
    "build_time": "2026-03-23T10:00:00Z",
    "tree_state": "clean"
  },
  "desired_state": {
    "repository": "file:///var/lib/core-ops/repo",
    "requested_ref": "main",
    "last_observed_revision": "a42be91",
    "last_observed_at": "2026-03-23T10:05:00Z"
  },
  "reconciliation": {
    "generation": 184,
    "status": "success",
    "running": false,
    "last_attempted_revision": "a42be91",
    "last_applied_revision": "a42be91",
    "last_started_at": "2026-03-23T10:06:00Z",
    "last_finished_at": "2026-03-23T10:06:09Z",
    "attempted_observed_divergence": null
  }
}
```

## Semantic Rules

- `schema_version` must be detectable before any other fields are trusted.
- The file is valid only if it is a complete snapshot on a supported schema version.
- Invalid, partial, or unsupported snapshots are ignored and treated as absent.
- Readers must observe the file as an atomic whole snapshot.
- `controller` contains identity data.
- `desired_state` contains observational data.
- `reconciliation` contains operational state.
- `generation` increases monotonically with each reconcile attempt.
- `status` must distinguish at minimum `in_progress`, `success`, and `failed`.
- `never_run` must be representable explicitly.
- If `last_attempted_revision` differs from `desired_state.last_observed_revision`, `attempted_observed_divergence` must explicitly represent that difference.
- On failed reconciliation, `last_applied_revision` remains the last successful applied revision.

## Never-Run Example

```json
{
  "schema_version": 1,
  "controller": {
    "version": "0.1.0",
    "revision": "8f3c2ab",
    "build_time": "2026-03-23T10:00:00Z",
    "tree_state": "clean"
  },
  "desired_state": {
    "repository": "file:///var/lib/core-ops/repo",
    "requested_ref": "main",
    "last_observed_revision": null,
    "last_observed_at": null
  },
  "reconciliation": {
    "generation": 0,
    "status": "never_run",
    "running": false,
    "last_attempted_revision": null,
    "last_applied_revision": null,
    "last_started_at": null,
    "last_finished_at": null,
    "attempted_observed_divergence": null
  }
}
```

## In-Progress Example

```json
{
  "schema_version": 1,
  "controller": {
    "version": "0.1.0",
    "revision": "8f3c2ab",
    "build_time": "2026-03-23T10:00:00Z",
    "tree_state": "clean"
  },
  "desired_state": {
    "repository": "file:///var/lib/core-ops/repo",
    "requested_ref": "main",
    "last_observed_revision": "c98dd10",
    "last_observed_at": "2026-03-23T10:07:00Z"
  },
  "reconciliation": {
    "generation": 185,
    "status": "in_progress",
    "running": true,
    "last_attempted_revision": "c98dd10",
    "last_applied_revision": "a42be91",
    "last_started_at": "2026-03-23T10:07:02Z",
    "last_finished_at": null,
    "attempted_observed_divergence": null
  }
}
```
