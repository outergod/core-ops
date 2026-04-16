# Quickstart: Systemd-Managed Host Agent

**Goal**: Run the GitOps Quadlet controller unattended via systemd service +
timer on a Fedora CoreOS host.

## Prerequisites

- Fedora CoreOS host with systemd and Quadlet support
- Git repository containing Quadlet files (container, socket, volume)
- Operator access to install systemd unit files

## Steps

1. Install `core-ops` on the host and initialize the controller state:
   ```bash
   install -m 0755 core-ops /usr/bin/core-ops
   core-ops init ssh://git@github.com/your-org/quadlets.git main
   ```
   This writes `/var/lib/core-ops/status.json` with the repository and tracking
   ref. The agent reads this on every run — no flags needed in the unit file.

2. Install the canonical unit files from `systemd/` in this repository:
   ```bash
   install -m 0644 core-ops.service /etc/systemd/system/core-ops.service
   install -m 0644 core-ops.timer /etc/systemd/system/core-ops.timer
   ```

3. To override the Quadlet directory or other defaults, use a drop-in
   (do not edit the unit file directly):
   ```bash
   systemctl edit core-ops.service
   ```
   Example drop-in to change the Quadlet directory:
   ```
   [Service]
   Environment=CORE_OPS_QUADLET_DIR=/etc/containers/systemd
   ```

4. Enable and start the timer:
   ```bash
   systemctl daemon-reload
   systemctl enable --now core-ops.timer
   ```

5. Confirm journald output includes plan/action summaries per run:
   ```bash
   journalctl -u core-ops.service -f
   ```

6. Update the Git repository and verify the agent converges to the new state.

## What to Expect

- Runs are scheduled by systemd timer and execute the oneshot service.
- Journald contains structured audit events for each run.
- Artifacts are reconciled in Volume → Container → Socket ordering, including
  container, socket, and volume Quadlets.
- Verification uses systemd unit state checks.

## Environment Overrides

You can override agent configuration with environment variables on the service
unit (via drop-in):

- `CORE_OPS_STATE_FILE` (default `/var/lib/core-ops/status.json`)
- `CORE_OPS_QUADLET_DIR` (default `/etc/containers/systemd`)
- `CORE_OPS_SYSTEMD_UNIT_DIR` (default `/etc/systemd/system`)
- `CORE_OPS_AUDIT_DIR` (optional)
- `CORE_OPS_LOCK_PATH` (optional)

The repository and tracking ref are set once via `core-ops init` and persisted
in the state file — they are not set via environment variables in the unit.

## Non-Goals

- Fleet orchestration or multi-host coordination
- Secret distribution
- Generic host configuration management
- Arbitrary environment file management as first-class objects
