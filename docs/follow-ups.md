# Follow-Ups

Deferred implementation work and discoveries that should be revisited after the active spec work is complete.

## CLI UX

### Init Command

> Historical note: the `init`-as-explicit-entry-point + remove-`repo`/`rev`
> redesign described here shipped in spec/015. Stateless `--source-repo`
> for plan/apply/explain shipped in spec/017. The remaining open items
> in this section are about argument persistence and recovery UX,
> below.

Other arguments currently taken by `plan`, `apply`, and `agent` which should persist are `quadlet-dir`, `systemd-unit-dir`, `state-file`, and `audit-dir`.

`rollback-plan-only` (apply option) is completely misplaced and should instead become the `rollback` option for `plan`.

There should be an explicit flow to re-initialize using `init`, e.g. using `--reinitialize` that changes the tracking repo and/or ref.

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

## NFS-backed library mounts in real workloads

Real homelab workloads (Immich photo library, Nextcloud data
directory, etc.) frequently back container volumes with NFS mounts
declared in `services/<svc>/systemd/*.mount` units. Spec/017's
`examples/03-immich/` uses a Podman-managed `*.volume` instead because
NFS mount declarations are orthogonal to the validation iteration's
scope. A future iteration could ship a worked example exercising
mount-aware reconciliation against an NFS source. (Spec/017 synthesis
table classification: C.)

## Source Repository UX

> The `Source repository` vs `workload Git repository` naming gap and the
> remaining authoring-tool follow-ups below. Spec/016 + spec/017 closed
> the "rich, documented real-life examples" and "QnA for known
> limitations" bullets — see `examples/<NN-slug>/` and the synthesis
> table at `specs/017-real-world-validation/spec.md`.

There should ideally be:
- User facing documentation how to author valid source repositories
- Agentic documentation for the same
- An installable Agent skill that teaches agents how to deal with source repositories
- A core-ops command that creates a source repository with basic layout, README.md and AGENTS.md from scratch (maybe plus the skill)

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
