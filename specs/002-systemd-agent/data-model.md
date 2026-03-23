# Data Model: Systemd-Managed Host Agent

## Entities

### HostAgentRun
- **Fields**: run_id, mode, status, started_at, finished_at, failure_class
- **Relationships**: references a ReconciliationPlan and AuditEvent
- **Validation**: must always record outcome (success/failure)

### QuadletArtifact
- **Fields**: name, artifact_type (container | socket | volume), unit_file_name,
  contents, desired_state
- **Relationships**: referenced by ReconciliationPlan actions
- **Validation**: artifact_type must be supported; name must be unique per type

### ReconciliationPlan
- **Fields**: plan_id, desired_revision_id, observed_revision_id, ordered_actions
- **Validation**: actions ordered Volume → Container → Socket

### VerificationResult
- **Fields**: artifact_name, artifact_type, unit_state, passed, message
- **Validation**: unit_state derived from systemd query

### AuditEvent
- **Fields**: run_id, plan_id, action_count, summary, timestamp
- **Validation**: emitted for every run under systemd

### RunLock
- **Fields**: lock_id, acquired_at, owner
- **Validation**: only one active lock per host

## Relationships

- HostAgentRun → ReconciliationPlan (1:1)
- HostAgentRun → AuditEvent (1:1)
- ReconciliationPlan → QuadletArtifact (1:N)
- QuadletArtifact → VerificationResult (1:1 per run)
