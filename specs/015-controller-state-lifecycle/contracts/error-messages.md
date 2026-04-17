# Contract: Error Messages for State Lifecycle

**Feature**: 015-controller-state-lifecycle

---

## Uninitialized State (state file absent)

All commands except `init` and `status` MUST fail with this message pattern:

```
controller is not initialized; run 'core-ops init <repository> <ref>' to initialize
```

Exit code: `1`

---

## Corrupt State (state file present but invalid)

All commands except `init` MUST fail with this message pattern:

```
state file at {path} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover
```

- `{path}` MUST be the absolute filesystem path to the state file
- This error MUST be distinguishable from the Uninitialized error
- Exit code: `1`

---

## Status Command Behavior

`status` does not fail on lifecycle problems; it reports them:

| Condition | Status output |
|---|---|
| Uninitialized | Reports state as `uninitialized`; no error exit |
| Corrupt | Reports state as `corrupt` with path; no error exit |
| Detached | Reports state as `detached` with currently applied detached revision |

---

## Detached State — Agent Behavior

When `agent` detects `detached = true`:

```
controller is detached at revision {revision}; apply and agent reconciliation are paused until re-attached via 'core-ops init <repository> <ref> --force'
```

Agent exits cleanly (exit code `0`) without performing reconciliation.

---

## Detached State — `plan` Command

`plan` operates normally in Detached mode. Its output MUST include a header indicating detached context:

```
[DETACHED] plan computed against detached revision {revision}; this represents what re-attaching to {requested_ref} would apply
```

---

## Rollback Rejection Messages

| Rejection reason | Message |
|---|---|
| `MissingSnapshot` | `no retained snapshot found for revision {rev}` |
| `IncompatibleScope` | `snapshot for revision {rev} was recorded on scope {snapshot_scope}, which is incompatible with current scope {current_scope}` |
| `Expired` | `snapshot for revision {rev} has expired from the rollback retention window` |
