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

The host agent is designed to run as a oneshot service triggered by a timer.
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
