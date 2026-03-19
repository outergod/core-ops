# Contract: Systemd Service + Timer

## Service Unit (oneshot)

- Purpose: Execute a single reconciliation run.
- Inputs: repo source, revision, quadlet directory, optional audit export path.
- Output: journald audit events plus operator-facing report.
- Exit codes: non-zero on failure; failure class logged to journald.

## Timer Unit

- Purpose: Trigger the oneshot service on a schedule.
- Behavior: Must not allow overlapping runs; timer re-triggers only after the
  previous run finishes.

## CLI Invocation Contract

- Service unit MUST call the CLI with explicit repo and revision.
- Timer unit MUST reference the service unit, not the binary directly.
