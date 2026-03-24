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

## Native Mount Management Workflow

- Declare managed mounts in `services/<service>/service.yaml` using stable mount
  identities rather than raw paths as the only key.
- Use `requires_mounts` on the consuming service so CoreOps can materialize
  native dependency semantics directly into the generated unit configuration.
- Keep ordinary `.mount` behavior as the default. Set `automount: true` only
  for explicitly network-backed mounts such as NFS.
- Limit prepared-path metadata to the service-consumed mount target. Creating
  missing directories plus optional owner, group, and mode is supported for
  bounded mount targets; generic directory management is not.
- `core-ops plan` should show generated mount identities, dependency counts,
  and automount identities when present.
- `core-ops apply` prepares bounded target paths, writes `.mount` and optional
  `.automount` units, and activates automount-backed mounts through the
  `.automount` unit instead of starting the `.mount` unit directly.
- Removing a managed mount stops dependent managed services first and fails
  explicitly if the mount is still busy or cannot be removed cleanly.
