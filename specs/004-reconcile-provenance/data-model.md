# Data Model: Provenance and Reconciliation Revision Tracking

## Entities

### Controller Provenance
- **Purpose**: Identify the CoreOps build or artifact that performed observation and reconciliation.
- **Fields**:
  - `version` (string, optional if unavailable in exceptional builds)
  - `revision` (string, optional if unavailable)
  - `build_time` (timestamp or build identifier)
  - `tree_state` (`clean` | `dirty` | `unknown`)
- **Rules**:
  - This domain is identity data.
  - Missing optional fields must remain explicit rather than inferred.

### Desired-State Provenance
- **Purpose**: Describe what desired state source was observed for the current host.
- **Fields**:
  - `repository` (string)
  - `requested_ref` (string)
  - `last_observed_revision` (immutable revision identifier, optional before first successful observation)
  - `last_observed_at` (timestamp, optional before first successful observation)
- **Rules**:
  - This domain is observational data.
  - `last_observed_revision` is the immutable result of resolving `requested_ref` at observation time.
  - This iteration reports desired-state provenance at host scope only.

### Reconciliation Provenance
- **Purpose**: Describe the latest reconciliation attempt and its operational outcome.
- **Fields**:
  - `generation` (u64)
  - `status` (`never_run` | `in_progress` | `success` | `failed` | future non-collapsing values)
  - `last_attempted_revision` (immutable revision identifier, optional in `never_run` state)
  - `last_applied_revision` (immutable revision identifier, optional before first successful apply)
  - `last_started_at` (timestamp, optional before first attempt)
  - `last_finished_at` (timestamp, optional while `in_progress`)
  - `running` (boolean or equivalent explicit in-progress representation)
  - `attempted_observed_divergence` (optional structured value when attempted != most recently observed)
- **Rules**:
  - This domain is operational state.
  - `generation` increases monotonically with each reconcile attempt.
  - `status` must distinguish at minimum `in_progress`, `success`, and `failed`.
  - `never_run` must be explicit rather than inferred from missing fields.
  - If `last_attempted_revision != last_observed_revision`, the divergence must be explicit.
  - Failed reconciliation must preserve `last_applied_revision` from the last success.

### Persisted Provenance Snapshot
- **Purpose**: Canonical host-local persisted representation of provenance for this iteration.
- **Fields**:
  - `schema_version` (integer or explicit version token)
  - `controller` (Controller Provenance)
  - `desired_state` (Desired-State Provenance)
  - `reconciliation` (Reconciliation Provenance)
- **Rules**:
  - This is derivative state, not authoritative desired state.
  - It is valid only when complete and on a supported schema version.
  - Invalid, partial, or unsupported snapshots are ignored and treated as absent.
  - Readers must observe it as a complete atomic snapshot.

### Repository Cache
- **Purpose**: Optional local acceleration for source access or failure recovery.
- **Fields**:
  - `cache_root` (path)
  - `cached_revision` (optional immutable revision identifier)
  - `last_refresh_at` (optional timestamp)
- **Rules**:
  - Cache contents are not authoritative provenance.
  - Cache data may be discarded without violating reconstructibility.

## Relationships

- One **Persisted Provenance Snapshot** contains exactly one **Controller Provenance**, one **Desired-State Provenance**, and one **Reconciliation Provenance** record.
- **Desired-State Provenance** supplies the most recently observed revision referenced by **Reconciliation Provenance**.
- **Repository Cache** may support observation, but provenance remains reconstructible without it.

## State Transitions

### Reconciliation Status
- `never_run -> in_progress` on the first reconcile attempt.
- `in_progress -> success` when the attempt completes and applies successfully.
- `in_progress -> failed` when the attempt ends unsuccessfully.
- `success -> in_progress` on a later reconcile attempt.
- `failed -> in_progress` on a retry or later reconcile attempt.

### Revision Semantics
- Observation updates `last_observed_revision` and `last_observed_at`.
- Attempt start increments `generation` and sets `last_attempted_revision`.
- Successful apply updates `last_applied_revision` to the attempted revision.
- Failed apply leaves `last_applied_revision` unchanged.
- Divergence is explicit whenever `last_attempted_revision` differs from `last_observed_revision`.

## Validation Rules

- Snapshot must include a detectable schema version before interpretation.
- Snapshot is accepted only if structurally complete.
- `generation` must never decrease.
- `running = true` must be consistent with an in-progress status representation.
- `last_finished_at` must be absent while reconciliation is in progress.
- `last_applied_revision` must not change on failed reconciliation.
