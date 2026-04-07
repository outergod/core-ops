# Verification Serial Console Readiness Proposal

## Purpose

Capture a minimal follow-on enhancement for the verification harness: replace
ARP-first guest IP discovery with a guest self-reported readiness signal emitted
on the serial console.

This is intentionally separated from the current accepted-corpus backfill work.

## Problem

The current VM-backed verification flow can depend on ARP-based guest IP
discovery. That mechanism is unreliable because it depends on:

- guest traffic appearing at the right time
- host neighbor-cache state
- bridge and L2 topology visibility
- timing that is outside the harness contract

This makes otherwise healthy verification runs fail or stall for reasons that do
not reflect the guest's real readiness.

## Proposed Minimal Change

Treat guest self-report on the serial console as the authoritative readiness and
IP discovery path for VM-backed verification runs.

### Minimal Design

1. The harness generates a per-run token and run id.
2. Ignition templates install a small readiness script and a oneshot systemd
   service in the guest.
3. After `network-online.target`, that service emits one structured readiness
   line to the guest serial console.
4. The host-side harness tails or reads the libvirt console log and waits for a
   readiness line whose token matches the current run.
5. The harness uses the reported IP as the authoritative guest address for SSH
   and later guest-boundary work.

## Why This Is The Smallest Practical Version

- no persistent guest agent
- no repo-under-test changes
- no dependency on SSH before readiness
- no shared-disk guest-to-host channel
- integrates naturally with existing libvirt console-log artifact capture

## Guest Payload

Ignition should write:

- `/usr/local/bin/core-ops-verify-ready`
- `core-ops-verify-ready.service`

### Service Shape

- `Type=oneshot`
- `After=network-online.target`
- `Wants=network-online.target`
- `ExecStart=/usr/local/bin/core-ops-verify-ready`

### Script Shape

The script should:

- read the injected run id and token
- determine the primary guest IPv4 address
- optionally include hostname and timestamp
- emit exactly one structured console line, for example:

```text
CORE_OPS_VERIFY_READY {"run_id":"run-123","token":"abc123","ip":"192.0.2.10","hostname":"vm-1","ts":"2026-04-07T00:00:00Z"}
```

## Host-Side Harness Behavior

The harness should:

- pass the run id and token into the guest through Ignition templating
- wait for the matching readiness line in the serial console log
- reject stale readiness lines whose token or run id do not match
- use the reported IP as the primary guest address
- keep ARP only as an optional fallback during migration, if needed

## Success Criteria

- VM-backed verification no longer depends primarily on ARP for guest IP
  discovery
- a healthy guest can become reachable without requiring opportunistic network
  neighbor observation
- stale console output cannot satisfy readiness for a new run
- timeout behavior is explicit when the readiness line never appears

## Non-Goals

- general-purpose guest agent framework
- persistent guest-host control channel
- cross-feature runtime telemetry system
- replacing later SSH readiness checks entirely

## Suggested Spec Delta

This should be added as a focused extension to
`008-e2e-verification-harness`, not as a broad new feature.

Suggested additions:

- the VM-backed harness SHOULD prefer guest self-reported readiness over
  ARP-derived address inference
- the guest readiness record MUST be bound to the current run identity
- failure to receive the readiness record within the configured readiness window
  MUST produce an explicit timeout or infrastructure outcome

## Suggested Task Slice

1. Extend ignition templating to inject a run-scoped readiness script and
   service.
2. Add host-side console-log parsing for the structured readiness record.
3. Use the reported IP as the primary VM-backed guest address source.
4. Retain ARP only as an optional fallback during rollout, or remove it if the
   new path proves sufficient.
5. Add verification-harness tests for:
   - successful readiness self-report
   - stale-token rejection
   - missing readiness record timeout
   - malformed readiness record handling
