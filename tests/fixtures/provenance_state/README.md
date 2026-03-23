# Provenance State Fixtures

Canonical persisted provenance snapshot fixtures for feature 004.

Files:
- `valid-success.json`: complete supported snapshot for a successful reconcile
- `valid-never-run.json`: explicit never-run snapshot
- `invalid-partial.json`: truncated/partial snapshot that must be treated as absent
- `invalid-unsupported-schema.json`: complete snapshot with unsupported schema version
