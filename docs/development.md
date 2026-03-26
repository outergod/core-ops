# Development (Nix + direnv)

This project assumes a Nix shell provided by `shell.nix` and loaded via direnv.
Do not assume Rust tooling is installed globally.

## Setup

1. Install direnv and Nix.
2. Run `direnv allow` at repo root.

## Common Commands

- Format: `cargo fmt` (or `make fmt`)
- Lint: `cargo clippy --all-targets --all-features -- -D warnings` (or `make lint`)
- Test: `cargo test` (or `make test`)

Assume the nix shell is already active, and do not run commands via `direnv exec`.

## Systemd Agent Configuration

The CoreOps host agent is designed to run as a oneshot service triggered by a timer.
Use a systemd drop-in to configure the repo source and revision without editing
unit files in place. The contract units are named `core-ops.service` and
`core-ops.timer`.

```
systemctl edit core-ops.service
```

Suggested drop-in content:

```
[Service]
Environment=CORE_OPS_REPO=ssh://git@github.com/your-org/quadlets.git
Environment=CORE_OPS_REV=main
Environment=CORE_OPS_QUADLET_DIR=/etc/containers/systemd
Environment=CORE_OPS_SYSTEMD_UNIT_DIR=/etc/systemd/system
```

Apply changes with:

- `systemctl daemon-reload`
- `systemctl restart core-ops.service`

Timer enablement example:

```
systemctl enable --now core-ops.timer
```

## Layered Overrides Development

Use the layered overrides fixture in `tests/fixtures/layered_overrides/` for
local testing. The repository layout should include:

- `services/<service>/` for base artifacts and base drop-ins
- `hosts/<host>/host.yaml` with explicit service selection
- `hosts/<host>/overrides/` for host-specific drop-ins

Override host selection during development with:

```
CORE_OPS_HOST=<host> core-ops plan --repo <repo> --rev <rev>
```

When adding or changing behavior, ensure tests and diagnostics preserve
machine-readable provenance for both the `core-ops` binary revision and the
desired-state revision being reconciled.

Any change that affects externally observable behavior, persisted state schema,
CLI output, reconciliation semantics, or compatibility must evaluate and
update the release version policy. The canonical controller version is the
package version in `Cargo.toml`.

## Provenance Status Snapshot Workflow

- Canonical persisted provenance defaults to
  `/var/lib/core-ops/status.json`.
- `--state-file <path>` or `CORE_OPS_STATE_FILE` override that default when a
  different path is required.
- `core-ops status` reads the canonical snapshot directly and treats missing,
  partial, invalid, or unsupported snapshots as absent.
- Apply and agent flows update the canonical snapshot by default rather than
  maintaining a parallel persisted view.
- `core-ops apply --force-no-state` is an explicit escape hatch for running an
  apply without updating the canonical snapshot. It is intended for exceptional
  cases, not normal operation.
- Backward-incompatible persisted-schema changes require a recorded version
  review and a controller version update in `Cargo.toml` according to the
  project versioning policy.


## Deterministic Reconciliation Workflow

- Deterministic reconciliation uses three normalized inputs for a managed scope:
  desired, `last_applied`, and observed actual state.
- `core-ops plan` remains the review surface for this model. Repeated planning
  with identical normalized inputs must produce materially identical action,
  drift, and dependency ordering output.
- `core-ops apply` only advances `last_applied` after side effects complete and
  post-apply verification reports convergence. Partial, blocked, repeated-
  failure, and oscillating outcomes keep the last known-good revision intact.
- `core-ops apply --rollback-to <revision>` reuses the same planner and
  dependency ordering as forward reconciliation. Use
  `--rollback-plan-only` before execution when reviewing disruptive changes.
- Retry is bounded. Repeated failure or oscillation for the same affected object
  set stops automatic progress and records structured convergence diagnostics for
  operator review.

### Normalization Rules and Tolerated Runtime Variance

Supported managed resource kinds in this iteration are generated systemd units,
Quadlet resources, managed mounts, managed automounts, and rendered host
artifacts.

- Generated systemd units
  - Normalize by canonical unit name and stable field ordering.
  - Treat effective unit content, dependency directives, and enablement-relevant
    semantics as material.
  - Ignore formatting-only differences and transient runtime state such as the
    currently active PID.
- Quadlet resources
  - Normalize by canonical resource filename and generated unit identity.
  - Treat semantically relevant section keys and rendered content as material.
  - Ignore ordering and whitespace differences that do not change generated
    systemd behavior.
- Managed mounts
  - Normalize by native `.mount` unit identity derived from `Where=`.
  - Treat source, target path, filesystem type, mount options, and
    CoreOps-managed preparation semantics as material.
  - Ignore runtime-only counters or other non-semantic mount statistics.
- Managed automounts
  - Normalize by native `.automount` unit identity derived from `Where=`.
  - Treat the automount path and CoreOps-managed dependency semantics as
    material.
  - Ignore runtime-only activation timing details once the effective automount
    contract matches desired state.
- Rendered host artifacts
  - Normalize by canonical target path and stable content representation.
  - Treat rendered content and ownership/path semantics managed by CoreOps as
    material.
  - Ignore non-semantic formatting differences introduced during rendering.

When a difference is intentionally ignored, it must be explainable as
`runtime_variance` rather than being silently dropped from operator-visible
reasoning.

## Native Mount Management Workflow

- Author managed mounts as native `.mount` and optional `.automount` artifacts
  and embed only reconciliation-specific metadata in an `[X-CoreOps]` section.
- Reference managed mounts from `services/<service>/service.yaml` by native
  `.mount` unit stem.
- Use `requires_mounts` on the consuming service so CoreOps can materialize
  native dependency semantics directly into the generated unit configuration.
- Keep ordinary `.mount` behavior as the default. Set `automount: true` only
  for explicitly network-backed mounts such as NFS.
- Keep `[X-CoreOps]` minimal in this iteration. `CreateMountpoint=true` is the
  default, and unsupported fields are rejected.
- `core-ops plan` should show native `.mount` stem references, dependency
  counts, and automount relationships when present.
- `core-ops apply` prepares bounded target paths, writes `.mount` and optional
  `.automount` units, and activates automount-backed mounts through the
  `.automount` unit instead of starting the `.mount` unit directly.
- Removing a managed mount stops dependent managed services first and fails
  explicitly if the mount is still busy or cannot be removed cleanly.

### Example Layout

```text
services/immich/
  immich.container
  service.yaml
  var-lib-immich-media.mount
  var-lib-immich-media.automount
```

`services/immich/service.yaml`

```yaml
requires_mounts:
  - var-lib-immich-media
```

`services/immich/var-lib-immich-media.mount`

```ini
[Unit]
After=network-online.target
Wants=network-online.target

[Mount]
What=nas:/volume1/media
Where=/var/lib/immich/media
Type=nfs
Options=rw,hard,noatime

[X-CoreOps]
CreateMountpoint=true
```

Optional `services/immich/var-lib-immich-media.automount`

```ini
[Automount]
Where=/var/lib/immich/media
```

### `[X-CoreOps]` Field Reference

- `CreateMountpoint`
  - Optional boolean.
  - Default: `true`.
  - Applies to the native `Where=` path from the `.mount` unit.
  - `true`: CoreOps creates the mountpoint directory if it is missing.
  - `false`: reconciliation fails if the mountpoint directory is missing.

### Override Rules

- Service-referenced managed mounts are keyed by native `.mount` stem.
- For those mounts, host overrides must not change the effective `Where=`
  value, because the stem is derived from that path.
- Host overrides may still change other native unit details such as `What=` or
  mount options, as long as the resulting layered unit remains valid.
- `[X-CoreOps]` follows the same layering order as native unit content. Later
  effective values override earlier ones before CoreOps validates the merged
  result.

### Troubleshooting

- `unsupported X-CoreOps field`
  - The `.mount` or `.automount` artifact still contains removed metadata.
  - Remove everything except `CreateMountpoint` from `[X-CoreOps]`.

- `mount unit name does not match Mount Where`
  - The `.mount` filename does not match the escaped systemd name derived from
    `Where=`.
  - Rename the file or fix `Where=` so they match.

- `automount unit name does not match Automount Where`
  - The `.automount` filename does not match the escaped systemd name derived
    from `Where=`.
  - Rename the file or fix `Where=` so they match.

- `mountpoint missing and CreateMountpoint=false`
  - CoreOps is configured not to create the mountpoint.
  - Provision the directory out of band, or set `CreateMountpoint=true`.
