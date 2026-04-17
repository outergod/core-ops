# Contract: CLI Command Changes

**Feature**: 015-controller-state-lifecycle

---

## Removed Arguments

The following arguments are removed from their respective commands. Operators who pass them will receive a "unexpected argument" error from clap.

| Command | Removed arguments |
|---|---|
| `core-ops plan` | `--repo`, `--rev` |
| `core-ops apply` | `--repo`, `--rev` |
| `core-ops agent` | `--repo`, `--rev` |
| `core-ops explain` | `--repo`, `--rev` |

---

## Replacement Behavior

All four commands source `repository` and `ref` exclusively from persisted controller state (`desired_state.repository`, `desired_state.requested_ref`).

If these fields are absent or the state file is absent/corrupt, the command fails with the appropriate lifecycle error (see `error-messages.md`).

---

## `core-ops plan` — Detached Mode

When the controller is in Detached state:

- `plan` MUST proceed normally (resolve `requested_ref` to current HEAD, compute plan using `last_applied_revision` as baseline)
- `plan` MUST include a detached-mode header in output (see error-messages contract)
- `plan` MUST NOT fail or refuse to run

---

## `core-ops apply` — Detached Mode

When the controller is in Detached state:

- `apply` MUST NOT perform reconciliation against `requested_ref`
- `apply` MUST fail with the detached-state error message
- Exception: `apply --rollback-to` remains valid from Detached; a further rollback leaves the controller Detached with the new revision

---

## `core-ops agent` — Detached Mode

When the controller is in Detached state:

- `agent` runs on its normal schedule
- `agent` detects `detached = true` at startup
- `agent` emits the detached status message
- `agent` exits cleanly (`0`) without performing reconciliation

---

## `core-ops status` — New Fields

`status` MUST expose:

- `desired_state.repository`
- `desired_state.requested_ref`
- `desired_state.last_observed_revision`
- `reconciliation.last_applied_revision`
- Lifecycle state (derived — Uninitialized / Corrupt / Initialized / Reconciling / Converged / Diverged / Detached)
- Detached revision (when Detached)

---

## `core-ops init` — New Command

See `cli-init.md` for the full contract.

`init` is added to `Commands` enum in `src/cli/args.rs`:

```rust
Commands::Init(InitArgs)
```

---

## Help Text Updates

All `--after_help` constants for affected commands must be updated to remove examples that include `--repo`/`--rev`. New examples must use initialized-state invocations:

```
core-ops plan
core-ops apply
core-ops apply --rollback-to <rev>
core-ops explain container/frontend.container
```
