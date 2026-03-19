# Quickstart: Systemd-Managed Host Agent

**Goal**: Run the GitOps Quadlet controller unattended via systemd service +
timer on a Fedora CoreOS host.

## Prerequisites

- Fedora CoreOS host with systemd and Quadlet support
- Git repository containing Quadlet files (container, socket, volume)
- Operator access to install systemd unit files

## Steps

1. Install the provided oneshot service and timer unit files on the host.
2. Enable the timer so it triggers reconciliation on the desired cadence.
3. Confirm journald output includes plan/action summaries per run.
4. Update the Git repository and verify the agent converges to the new state.

## What to Expect

- Runs are scheduled by systemd timer and execute the oneshot service.
- Journald contains structured audit events for each run.
- Artifacts are reconciled in Volume → Container → Socket ordering.
- Verification uses systemd unit state checks.

## Non-Goals

- Fleet orchestration or multi-host coordination
- Secret distribution
- Generic host configuration management
- Arbitrary environment file management as first-class objects
