# Quickstart: Controller State Model and Lifecycle

**Feature**: 015-controller-state-lifecycle  
**Date**: 2026-04-14

---

## What Changes

This feature introduces the `core-ops init` command and removes `--repo`/`--rev` from `plan`, `apply`, `agent`, and `explain`. After this change, CoreOps has an explicit lifecycle: operators initialize once, then all reconciliation commands source their configuration from persisted state.

---

## Implementation Order

### Step 1: Add `StateError::Corrupt` and fix `read_persisted_state`

**File**: `src/core/errors.rs`

Add `Corrupt(String)` variant:

```rust
#[error("state file is corrupt: {0}")]
Corrupt(String),
```

**File**: `src/io/state.rs`

Change `read_persisted_state` to return `Err(StateError::Corrupt(...))` when the file exists but `parse_persisted_state_text` returns `None`:

```rust
pub fn read_persisted_state(path: &Path) -> Result<Option<PersistedProvenanceState>, StateError> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(StateError::Io(err.to_string())),
    };
    match parse_persisted_state_text(&contents) {
        Some(state) => Ok(Some(state)),
        None => Err(StateError::Corrupt(path.display().to_string())),
    }
}
```

**After**: Audit all callers. The `.ok().flatten()` pattern now silently discards `Corrupt` — safe only for post-apply best-effort audit emission. Pre-action callers must handle `Corrupt` with an actionable error.

---

### Step 2: Add `detached` flag to `PersistedProvenanceState`

**File**: `src/core/types.rs`

```rust
#[serde(default)]
pub detached: bool,
```

Add to `PersistedProvenanceState`. No schema version change needed.

---

### Step 3: Add `InitArgs` and `Commands::Init` 

**File**: `src/cli/args.rs`

Add `InitArgs` struct with positional `repository`, positional `requested_ref`, and `--force` flag. Add `Commands::Init(InitArgs)` to the `Commands` enum.

Remove `--repo`/`--rev` from `PlanArgs`, `ApplyArgs`, `AgentArgs`, `ExplainArgs`.

Update all `--after_help` strings to remove examples that include `--repo`/`--rev`.

---

### Step 4: Implement `src/cli/init.rs`

Implement `run_init(config: &InitConfig) -> Result<(), CoreError>`:

1. Resolve the state file path
2. Check for existing state:
   - `Ok(None)` → proceed
   - `Ok(Some(_))` without `--force` → fail with "already initialized" error
   - `Err(StateError::Corrupt(_))` without `--force` → fail with corrupt-state error (direct to `--force`)
   - `Err(StateError::Io(_))` → propagate
3. Validate `requested_ref` against the repository (branch or tag only)
4. Write new `PersistedProvenanceState` with `NeverRun` reconciliation and `detached = false`
5. On `--force` with unchanged repo/ref: preserve existing reconciliation and deterministic state; clear detached flag
6. On `--force` with changed repo/ref: write fresh state; MAY clear deterministic state

---

### Step 5: Update `src/cli/agent.rs`

1. Remove `repo`/`rev` from `AgentConfig`
2. Replace `persist_never_run_state` bootstrap with a lifecycle check:
   - `Ok(None)` → fail with "controller not initialized" error
   - `Err(StateError::Corrupt(_))` → fail with corrupt-state error
   - `Ok(Some(state))` where `state.detached = true` → emit detached message, exit cleanly
   - `Ok(Some(state))` → proceed with reconciliation using `state.desired_state.repository` and `state.desired_state.requested_ref`
3. Remove `CORE_OPS_REPO`/`CORE_OPS_REV` env var resolution

---

### Step 6: Update `src/cli/plan.rs`

Remove `repo`/`rev` parameters from plan invocation. Read from persisted state; fail with lifecycle errors on absent/corrupt.

---

### Step 7: Update `src/cli/apply.rs`

Remove `repo`/`rev` from apply invocation paths. Read from persisted state.

Handle `--rollback-to` in Detached mode: allowed; on success, writes new `last_applied_revision` and keeps `detached = true`.

Handle regular apply in Detached mode: MUST fail with the detached-state error.

---

### Step 8: Update `src/cli/explain.rs`

Remove `--repo`/`--rev` fallback from `resolve_explain_target`. Read exclusively from persisted state. Handle absent/corrupt state with lifecycle errors.

---

### Step 9: Update `src/cli/status.rs`

Add `Err(StateError::Corrupt(_))` arm to report corrupt state clearly. Expose the new required fields: lifecycle state, detached revision.

---

### Step 10: Update `src/main.rs`

1. Add `Commands::Init` dispatch
2. Remove `resolve_env(args.repo, "CORE_OPS_REPO")` and `resolve_env(args.rev, "CORE_OPS_REV")` calls for affected commands
3. Audit `.ok().flatten()` at line 161 — this is a post-apply audit call; it remains safe

---

### Step 11: Bump version and update changelog

**File**: `Cargo.toml`

```toml
version = "1.0.0"
```

**File**: `CHANGELOG.md`

Add Keep-a-Changelog entry under `[Unreleased]` with `### Breaking Changes` for `--repo`/`--rev` removal and `### Added` for `core-ops init`.

**File**: `changes/` (release intent fragment)

```
bump: major
reason: remove --repo/--rev from plan, apply, agent, explain (breaking CLI change); add core-ops init
```

---

## Validation Gate

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Both must pass with zero warnings treated as errors.

---

## Operator Migration

Operators running CoreOps 0.x must:

1. Run `core-ops init <repository> <ref>` once to initialize persisted configuration
2. Remove `--repo`/`--rev` from any systemd unit `ExecStart=` lines, scripts, or CI invocations
3. CoreOps 1.0.0 will refuse to start reconciliation without prior `init`
