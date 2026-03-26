# Deterministic Reconciliation Fixtures

Fixture scaffolding for feature 006 deterministic reconciliation.

Scenarios:
- `baseline/`: desired, last_applied, and actual state align and should produce a no-op plan
- `external-drift/`: actual state diverges from last_applied without a desired change
- `rollback/`: retained successful snapshots support rollback planning and partial rollback recording
- `oscillation/`: repeated attempts observe materially alternating actual state

These directories are Phase 1 scaffolding only. Concrete fixture payloads are added in later phases with the three-way planner, rollback, and convergence models.
