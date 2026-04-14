# Follow-Ups

Deferred implementation work and discoveries that should be revisited after the active spec work is complete.

## CLI UX

### Init Command

`core-ops` currently expects every `plan`, `apply` (or `agent`) to be supplied with `repo` and `rev` arguments. At the same time, expected use through operators is to initialize `core-ops` once against a repository and a tracking branch, and keep running `plan`, `apply` etc. against that.

`repository` and `requested_ref` are already tracked through `core-ops`, so `repo` should be taken from their and `rev` be assumed to be the latest tracking `requested_ref`. 

In their place, a new `init` command shall be introduced in the form of `init [repo] [ref]` that sets up `core-ops` state with the tracking repository and ref. At the same time, remove the `repo` and `rev` arguments from `plan` and `apply`, effectively making the CLI stateful and aligned with the state store.

Other arguments currently taken by `plan`, `apply`, and `agent` which should persist are `quadlet-dir`, `systemd-unit-dir`, `state-file`, and `audit-dir`.

Rollbacks would then be validated against `rev`s on the tracking branch, and otherwise refuse action if pointing to a non-reachable commit from the current ref.
`rollback-plan-only` (apply option) is completely misplaced and should instead become the `rollback` option for `plan`.

There should be an explicit flow to re-initialize using `init`, e.g. using `--reinitialize` that changes the tracking repo and/or ref.

Summary:
- CoreOps already persists tracking repository/ref in controller state
- CLI UX should be aligned with that existing persistence
- init becomes the explicit operator entry point for managing this persisted desired-state configuration

Read specs 004, 006, and 007 to get the full picture.

### Reconciliation Cleanup

Investigate the contents of status.json and deterministic-state.json to see whether state is duplicated. Consider removing state from status.json if duplicated.

### Help Text Fidelity

The current `help` command output is all over the place. Help output should encode *mental model*, not mere syntax.

```
GitOps controller for Quadlet, native systemd units, and mount-aware reconciliation

Usage: core-ops <COMMAND>

Commands:
  plan     Compute a reconciliation plan, including native .mount/.automount artifacts with minimal [X-CoreOps] metadata and generated dependency semantics
  apply    Apply a reconciliation plan, including CreateMountpoint-driven mountpoint preparation and mount-aware native unit activation
  agent    Run the agent once (intended for systemd service execution)
  status   Display canonical persisted provenance from a status snapshot, treating invalid or missing snapshots as absent
  explain  Explain a single managed object using the authoritative reconciliation model
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

License:
  GNU Affero General Public License version 3 or later (AGPLv3+)
```

- Summary at the top should be aligned with README: `Convergence engine for systemd-based hosts`
- `plan` and `apply` should be changed to `Compute desired vs actual state diff` and `Converge system to desired state`, respectively
- `status` shorten to `Display canonical persisted provenance`
- `explain` shorten to `Explain a managed object`
- Additional information that's otherwise lost should instead go into individual command `help` descriptions, e.g. `core-ops help plan` can explain details about mount artifacts, X-CoreOps metadata etc.

### Status Command Output

The output provided by `status` is currently machine-readable by default and there is no human-readable alternative. There should be an option to change the output type aligned with the other commands, and it should be humane by default.
In addition, the help for the `state-file` option is far too verbose.

### Non-root behavior

`core-ops` happily runs when using with an underprivileged user and will provide false or misleading information. For instance,
```
core-ops status
provenance
{
  "status": "absent"
}
```

Instead of a warning that the user doesn't have permission to read or operate on the status file.
`core-ops` should ideally check either:
1. Whether the running user has all necessary permissions to operate or at least
2. Just flat-out deny not running as non-root

For now, go with option 2.

## Source Repository UX

There is no user-facing and no agent-facing documentation for the required layout of the source Git repository. Even the naming is not aligned (`Source repository` vs `workload Git repository`).

There should ideally be:
- User facing documentation how to author valid source repositories
- Agentic documentation for the same
- An installable Agent skill that teaches agents how to deal with source repositories
- A core-ops command that creates a source repository with basic layout, README.md and AGENTS.md from scratch (maybe plus the skill)
- Important: Rich, documented real-life examples of actual source repositories with real services, overrides, mounts etc.
- QnA for source repository use cases / known limitations and workarounds

These changes should be structured around schema, patterns (conventions), and tooling.

## Installation UX

For better integration and install experience, provide:
- An RPM that also includes the systemd unit/timer
- An RPM repository that can easily (-> documentation) be added to install the artifacts from
- A warning why there is no `curl ... | sudo bash` convenience script (nobody should ever use that -> threat model)

## README asciinema

To give potential users an idea of what it's like to run `core-ops`, embed an asciinema that shows running `apply` on the front page of the project.

## Secrets UX

Integrate with `core-ops` one or more mechanisms including `podman secrets` and/or `systemd-credstore` to manage secrets. `core-ops` doesn't have to replace the mechanisms, but at least be aware of them when Quadlets reference secrets this way and be able to check for them and bail out with a meaningful error if they don't exist, and explain how to create them. Optionally, integrate secrets creation wrapping the two tools with `core-ops`.

Levels of integration:
1. Detection only
  - validate existence
  - fail clearly
2. Reference awareness
  - understand how Quadlets refer to secrets
3. Provisioning (optional)
  - create via wrapper
