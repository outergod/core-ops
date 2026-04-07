# Proposal: Verification Live Progress Streaming

## Problem

`core-ops-verify run` is still too quiet during long VM-backed execution.
Operators currently have limited visibility into whether the harness is:

- booting the guest
- waiting for serial-console readiness
- rejecting stale or malformed readiness records
- falling back to ARP during migration
- copying binaries or fixtures into the guest
- running `core-ops`
- blocked on a long guest command

This makes real runs feel hung even when they are progressing normally, and it
slows diagnosis when a run is stalled or about to time out.

## Goal

Add live operator-facing progress output for `core-ops-verify` that explains
what the harness is doing while execution is in flight, without changing the
authoritative final JSON contract.

## Non-Goals

- Do not make raw serial-console streaming the default behavior.
- Do not change the final machine-readable `verification_run` payload shape as
  the primary source of truth.
- Do not introduce a second independent spinner/progress implementation.
- Do not turn `core-ops-verify` into a general-purpose log viewer.

## Proposed UX

### Default non-interactive behavior

Keep current behavior:

- final humane report on stdout
- final JSON on stdout when `--json` is requested

### Interactive terminal behavior

When stdout is a terminal and JSON-only output is not being used, emit live
progress for the current scenario.

Minimal event set:

- `booting guest`
- `waiting for serial-console readiness`
- `rejected stale readiness record`
- `rejected malformed readiness record`
- `accepted readiness record: <ip>`
- `using migration fallback: arp`
- `copying core-ops binary`
- `copying repository fixture`
- `running core-ops apply`
- `running guest command <step-id>`
- `rebooting guest`
- terminal success/failure/timeout state for each step

### Heartbeat behavior

While waiting in long-running phases, show a heartbeat on the active spinner
line rather than printing a new log line every tick.

Examples:

- `waiting for serial-console readiness (12s, rejected: 2)`
- `running core-ops apply (37s)`
- `running guest command verify-status (91s)`

The heartbeat should update in place and preserve a single clean active line.

## Reuse Existing Spinner

This feature should reuse the existing interactive apply spinner rather than
inventing a new one.

Existing relevant code:

- [main.rs](/home/outergod/code/github.com/outergod/core-ops/src/main.rs)
  - `SpinnerHandle`
  - `InteractiveApplyDisplay`
- [report.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs)
  - `ApplyInteractiveEvent`

Recommended direction:

1. Extract the spinner and interactive display into a small shared verification-
   agnostic terminal-progress surface.
2. Add verification-specific event types, analogous to `ApplyInteractiveEvent`.
3. Reuse the same spinner frame cadence and line-replacement behavior for
   verification heartbeats.

This keeps operator experience consistent across:

- `core-ops apply`
- `core-ops-verify run`

## Proposed Event Model

Add a verification-specific interactive event stream, for example:

- `Begin(text)`
- `Started { phase, line }`
- `Heartbeat { phase, line }`
- `Info(block)`
- `Terminal { phase, block }`
- `Finish(text)`

Semantics:

- `Started` starts the spinner for the active phase
- `Heartbeat` updates the same spinner line in place
- `Info` prints one-off structured notes and then returns to spinner mode if a
  phase is still active
- `Terminal` clears the spinner and prints a completed/failed/timed-out block
- `Finish` prints the final summary

## Suggested Flags

Phase 1 minimal scope:

- no new flag required for basic interactive live progress
- reuse `--verbose` to print richer one-off details

Phase 2 optional expansion:

- `--stream-console`
  - opt-in raw serial-console streaming
- `--stream-guest-output`
  - opt-in passthrough for guest command stdout/stderr as it arrives

## Implementation Shape

### Phase A: Structured live progress

Add progress emission from `execute_scenario` and readiness acquisition:

- emit `Started` for boot/readiness/step execution
- emit `Heartbeat` while waiting for readiness or long guest commands
- emit `Info` when a readiness record is rejected or accepted
- emit `Terminal` on step completion/failure/timeout

Key touch points:

- [verification.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs)
- [libvirt.rs](/home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs)
- [main.rs](/home/outergod/code/github.com/outergod/core-ops/src/main.rs)
- [report.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs)

### Phase B: Optional raw-stream support

If needed later, layer opt-in raw stream support on top of the structured
progress model rather than replacing it.

## Output Contract

The live stream is operator UX, not the authoritative automation contract.

Requirements:

- final JSON output remains stable
- final humane report remains stable
- live progress is best-effort and terminal-oriented
- CI/non-interactive mode does not depend on live streaming for correctness

## Failure Semantics

Live progress should help users distinguish:

- still booting
- waiting for readiness
- rejecting stale/malformed readiness
- readiness timeout
- infrastructure failure after readiness
- behavioral CoreOps failure after command execution

It must not blur those categories or replace the final recorded run outcome.

## Acceptance Criteria

- Interactive VM-backed verification shows an active spinner and current phase.
- Waiting for readiness shows in-place heartbeat updates.
- Acceptance or rejection of readiness records is surfaced live.
- Long-running guest commands show an active phase with elapsed time.
- Final humane and machine-readable outputs remain unchanged in meaning.
- The implementation reuses the existing spinner behavior rather than adding a
  second terminal-progress subsystem.

## Suggested Task Slice

1. Extract shared spinner/display primitives from
   [main.rs](/home/outergod/code/github.com/outergod/core-ops/src/main.rs).
2. Define verification interactive event types in
   [report.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/report.rs)
   or a small adjacent module.
3. Emit live progress events from
   [verification.rs](/home/outergod/code/github.com/outergod/core-ops/src/cli/verification.rs).
4. Emit readiness heartbeat and acceptance/rejection info from
   [libvirt.rs](/home/outergod/code/github.com/outergod/core-ops/src/io/libvirt.rs).
5. Add interactive rendering tests analogous to the apply-path coverage.
6. Document the behavior in
   [development.md](/home/outergod/code/github.com/outergod/core-ops/docs/development.md).
