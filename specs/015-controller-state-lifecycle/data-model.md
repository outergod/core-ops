# Data Model: Controller State Model and Lifecycle

**Feature**: 015-controller-state-lifecycle  
**Date**: 2026-04-14

---

## Modified Types

### `StateError` (`src/core/errors.rs`)

Add a new variant to distinguish a corrupt-but-present state file from an I/O error:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StateError {
    #[error("state io error: {0}")]
    Io(String),
    #[error("state serialization error: {0}")]
    Serialization(String),
    #[error("state file is corrupt: {0}")]
    Corrupt(String),   // NEW: file exists but is not a valid complete snapshot
}
```

**Usage rule**: `Corrupt(path)` is returned when the state file is present on disk but fails deserialization or validation. It is never returned when the file is absent (absent → `Ok(None)`).

---

### `PersistedProvenanceState` (`src/core/types.rs`)

Add the detached flag with a default so existing state files without the field deserialize as `false`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProvenanceState {
    pub schema_version: u32,
    pub controller: ControllerProvenance,
    pub desired_state: DesiredStateProvenance,
    pub reconciliation: ReconciliationProvenance,
    #[serde(default)]
    pub detached: bool,   // NEW: true only after a successful snapshot rollback
}
```

**Invariants**:
- `detached = true` is only valid when `reconciliation.status` is `Success` or `Failed`
- `detached = true` is never valid when `reconciliation.status` is `NeverRun` or `InProgress`
- `init --force` MUST write `detached = false`
- Existing files without the field deserialize as `detached = false` (not detached)

---

### `ReconciliationProvenance` (`src/core/types.rs`)

No structural changes. The `detached` flag lives on `PersistedProvenanceState` rather than here, because it is a controller lifecycle flag, not a reconciliation outcome field.

---

## New Types

### `InitArgs` (`src/cli/args.rs`)

New args struct for the `core-ops init` subcommand:

```rust
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Source repository (local path or Git URL).
    pub repository: String,
    /// Branch or tag name to track.
    pub requested_ref: String,
    /// Overwrite existing configuration.
    #[arg(long)]
    pub force: bool,
}
```

**Validation rules** (enforced in `src/cli/init.rs`):
- `repository` must be reachable
- `requested_ref` must resolve to a branch (`refs/heads/`) or tag (`refs/tags/`) in the given repository
- Bare commit hashes and other non-symbolic refs are rejected with a named error

---

## `read_persisted_state` Behavior Change (`src/io/state.rs`)

Before:
```
absent    → Ok(None)
invalid   → Ok(None)    ← silent
I/O error → Err(StateError::Io)
```

After:
```
absent    → Ok(None)
invalid   → Err(StateError::Corrupt(path))   ← explicit
I/O error → Err(StateError::Io)
```

The `parse_persisted_state_text` function signature is unchanged (returns `Option<PersistedProvenanceState>`).

---

## Removed Fields

### `AgentConfig` (`src/cli/agent.rs`)

Remove:
```rust
pub repo: String,   // REMOVED
pub rev: String,    // REMOVED
```

Repository and ref are read from persisted state at runtime.

### `AgentArgs` (`src/cli/args.rs`)

Remove:
```rust
pub repo: Option<String>,   // REMOVED
pub rev: Option<String>,    // REMOVED
```

### `PlanArgs` (`src/cli/args.rs`)

Remove:
```rust
pub repo: String,   // REMOVED
pub rev: String,    // REMOVED
```

### `ApplyArgs` (`src/cli/args.rs`)

Remove:
```rust
pub repo: String,   // REMOVED
pub rev: String,    // REMOVED
```

### `ExplainArgs` (`src/cli/args.rs`)

Remove:
```rust
pub repo: Option<String>,   // REMOVED
pub rev: Option<String>,    // REMOVED
```

---

## Controller Lifecycle State Derivation

The lifecycle state is a derived value — not stored directly. It is computed from observable conditions:

| State         | Condition |
|---|---|
| Uninitialized | State file absent (`Ok(None)`) |
| Corrupt       | State file present, read returns `Err(StateError::Corrupt(_))` |
| Initialized   | Valid state; `reconciliation.status = NeverRun` |
| Reconciling   | Valid state; `status = InProgress`; `running = true`; `last_started_at` set; `last_finished_at` absent |
| Converged     | Valid state; `status = Success`; `last_applied_revision == last_attempted_revision`; `detached = false` |
| Diverged      | Valid state; `status = Failed`; `running = false`; `detached = false` |
| Detached      | Valid state; `detached = true`; `status = Success` or `Failed` |

Derivation logic lives in a new pure function, e.g. `fn lifecycle_state(state: &PersistedProvenanceState) -> LifecycleState`, or equivalently derived at each call site.

---

## Scope Identity (no change from existing implementation)

Format: `host:<hostname>:<machine-id>`
- hostname: `CORE_OPS_HOST` env var if set, otherwise system hostname
- machine-id: contents of `/etc/machine-id`

Each `RetainedAppliedSnapshot.scope_id` carries this value. No schema change required.
