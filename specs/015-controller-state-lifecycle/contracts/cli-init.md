# Contract: `core-ops init` Command

**Feature**: 015-controller-state-lifecycle

---

## Command Signature

```
core-ops init <repository> <requested_ref> [--force]
```

| Argument | Type | Required | Description |
|---|---|---|---|
| `<repository>` | positional string | yes | Local path or Git URL of the source repository |
| `<requested_ref>` | positional string | yes | Branch name or tag name to track |
| `--force` | flag | no | Overwrite existing configuration |

---

## Preconditions

| Condition | Outcome |
|---|---|
| State file absent | Initialization proceeds normally |
| State file present, valid, `--force` absent | Fails with `already-initialized` error |
| State file present, corrupt, `--force` absent | Fails with `corrupt-state` error (same message as other commands; directs operator to `--force`) |
| State file present, `--force` present | Reinitializes; see Reinitialization rules below |

---

## Validation

1. `<requested_ref>` is resolved in `<repository>`.
2. The resolved ref MUST be a branch (`refs/heads/…`) or tag (`refs/tags/…`).
3. A bare commit hash or other non-symbolic ref MUST be rejected:
   - Error message: `"<ref>" is not a valid tracking ref; only branch and tag names are accepted`

---

## Directory Creation

If the parent directory of the state file path does not exist, `init` MUST create it (including any missing intermediate directories) before writing the state file. This matches the behavior of `write_persisted_state` via `fs::create_dir_all`.

---

## Success Behavior

On success, `init` writes a `PersistedProvenanceState` with:

- `desired_state.repository` = `<repository>`
- `desired_state.requested_ref` = `<requested_ref>`
- `desired_state.last_observed_revision` = absent (`None`)
- `desired_state.last_observed_at` = absent (`None`)
- `reconciliation.status` = `NeverRun`
- `detached` = `false`
- All other reconciliation fields at zero/absent values

---

## Reinitialization (`--force`) Rules

### When `<repository>` and `<requested_ref>` are UNCHANGED (re-attach):

- Overwrite `desired_state.repository` and `desired_state.requested_ref` (same values)
- Clear `detached` flag (set to `false`)
- **Preserve** all reconciliation history (`reconciliation.*`)
- **Preserve** deterministic state (retained snapshots)

### When `<repository>` or `<requested_ref>` CHANGED:

- Overwrite `desired_state.repository` and `desired_state.requested_ref`
- Clear `detached` flag (set to `false`)
- Reset `reconciliation` to `NeverRun` state (clear all reconciliation fields)
- Clear retained snapshots in `deterministic-state.json` (snapshots recorded for the previous tracking configuration are not meaningful for the new one and MUST be discarded to prevent cross-configuration rollback confusion)

**Rationale for clearing**: Retained snapshots encode the managed state of a specific host under a specific tracking configuration. Carrying them across a repository or ref change would allow accidental rollback into a state never applied under the new configuration, producing undefined behavior.

---

## Error Messages

| Situation | Error |
|---|---|
| Already initialized, no `--force` | `controller is already initialized (repository: {repo}, ref: {ref}); use --force to reinitialize` |
| State file corrupt, no `--force` | `state file at {path} is corrupt or unreadable; run 'core-ops init <repository> <ref> --force' to recover` |
| Ref is a commit hash | `"{ref}" is not a valid tracking ref; only branch and tag names are accepted` |
| Ref not resolvable | `ref "{ref}" could not be resolved in repository {repo}: {detail}` |
| Repository unreachable | `repository {repo} could not be opened: {detail}` |

---

## Exit Codes

| Outcome | Exit code |
|---|---|
| Success | `0` |
| Validation failure | `1` |
| Already initialized | `1` |
| Corrupt state (without `--force`) | `1` |
