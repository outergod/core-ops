# Quickstart: GitOps Quadlet Controller (MVP)

**Goal**: Reconcile Quadlet workloads from a Git repository onto a single Fedora
CoreOS host with safe, observable changes.

## Prerequisites

- Fedora CoreOS host with systemd and Quadlet support
- Git repository containing Quadlet unit files (MVP assumes a single host)
- Operator access to run the controller on the host

## Steps

1. Prepare a Git repository with valid Quadlet unit files under the agreed
   repository directory.
2. Run the controller in plan mode and review the proposed actions and diffs.
3. If the plan is acceptable, run the controller in apply mode with explicit
   operator intent.
4. Review the reconciliation outcome and audit output to confirm convergence.

## What to Expect

- The controller writes Quadlet files to the system location and reloads systemd.
- Generated systemd units are enabled/disabled or started/stopped as needed.
- All changes are visible via audit output and status reporting.

## Non-Goals (MVP)

- Fleet management or multi-host orchestration
- Secret distribution
- Full host configuration management
