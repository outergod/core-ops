# Quickstart: Systemd-Managed Host Agent

**Goal**: Run the GitOps Quadlet controller unattended via systemd service +
timer on a Fedora CoreOS host.

## Prerequisites

- Fedora CoreOS host with systemd and Quadlet support
- Git repository containing Quadlet files (container, socket, volume)
- Operator access to install systemd unit files

## Steps

1. Install the provided oneshot service and timer unit files on the host.
   - Copy `specs/002-systemd-agent/contracts/systemd/core-ops.service` to
     `/etc/systemd/system/core-ops.service`
   - Copy `specs/002-systemd-agent/contracts/systemd/core-ops.timer` to
     `/etc/systemd/system/core-ops.timer`
2. Configure the repo and revision the agent should reconcile.
   - Use `systemctl edit core-ops.service` and add:
     ```
     [Service]
     Environment=CORE_OPS_REPO=ssh://git@github.com/your-org/quadlets.git
     Environment=CORE_OPS_REV=main
     Environment=CORE_OPS_QUADLET_DIR=/etc/containers/systemd
     Environment=CORE_OPS_SYSTEMD_UNIT_DIR=/etc/systemd/system
     ```
3. Enable and start the timer:
   - `systemctl daemon-reload`
   - `systemctl enable --now core-ops.timer`
4. Confirm journald output includes plan/action summaries per run:
   - `journalctl -u core-ops.service -f`
5. Update the Git repository and verify the agent converges to the new state.

## What to Expect

- Runs are scheduled by systemd timer and execute the oneshot service.
- Journald contains structured audit events for each run.
- Artifacts are reconciled in Volume → Container → Socket ordering, including
  container, socket, and volume Quadlets.
- Verification uses systemd unit state checks.

## Environment Overrides

You can override the agent configuration with environment variables on the
service unit:

- `CORE_OPS_REPO` (required)
- `CORE_OPS_REV` (required)
- `CORE_OPS_QUADLET_DIR` (default `/etc/containers/systemd`)
- `CORE_OPS_SYSTEMD_UNIT_DIR` (default `/etc/systemd/system`)
- `CORE_OPS_AUDIT_DIR` (optional)
- `CORE_OPS_LOCK_PATH` (optional)

## Non-Goals

- Fleet orchestration or multi-host coordination
- Secret distribution
- Generic host configuration management
- Arbitrary environment file management as first-class objects
