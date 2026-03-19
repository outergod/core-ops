# Quickstart: GitOps Quadlet Controller (MVP)

**Goal**: Reconcile Quadlet workloads from a Git repository onto a single Fedora
CoreOS host with safe, observable changes.

## Prerequisites

- Fedora CoreOS host with systemd and Quadlet support
- Git repository containing Quadlet unit files (MVP assumes a single host)
- Operator access to run the controller on the host

## Steps

1. Prepare a Git repository with valid Quadlet unit files under `quadlets/`.
2. Run plan mode with an explicit revision:

   ```bash
   core-ops plan --repo <git-url-or-path> --rev <branch|tag|commit>
   ```

3. If the plan is acceptable, apply it:

   ```bash
   core-ops apply --repo <git-url-or-path> --rev <branch|tag|commit>
   ```

4. Review the reconciliation outcome and audit output to confirm convergence.

## What to Expect

- The controller reads Quadlet files from the repo `quadlets/` directory.
- The default system Quadlet path is `/etc/containers/systemd` unless overridden
  with `--quadlet-dir`.
- The controller writes Quadlet files to the system location and reloads systemd.
- Generated systemd units are started/stopped as needed.
- Enablement is expressed via Quadlet [Install] sections, not systemctl enable.
- Audit events are emitted to the systemd journal when available.
- All changes are visible via audit output and status reporting.

## Non-Goals (MVP)

- Fleet management or multi-host orchestration
- Secret distribution
- Full host configuration management
