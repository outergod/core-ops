# Quickstart: Serial Console Readiness

## Purpose

Show the intended development and validation workflow for introducing
serial-console readiness as the primary VM-backed guest readiness path.

## Scope

This feature changes how the VM-backed verification harness determines the
guest's authoritative IPv4 address before later SSH and guest-boundary steps.
It does not change the behavioral CoreOps actions exercised after the guest is
ready.

## Development Workflow

1. Update the VM-backed provisioning path so each guest receives a run-scoped
   readiness payload.
2. Emit a structured readiness record on the guest serial console after network
   readiness.
3. Update the host-side harness to parse console logs, match the current run
   identity, and select the guest IPv4 address from the accepted readiness
   record.
   The first valid current-run record wins; later matching records are kept
   only as diagnostics.
4. Preserve ARP-based discovery only as a temporary migration fallback, used
   only when no valid readiness record has been accepted and rollout fallback
   has been explicitly enabled.
5. Export readiness evidence in retained artifacts and machine-readable run
   outputs.

## Validation Workflow

1. Run deterministic unit and integration coverage for readiness parsing,
   stale-record rejection, malformed-record rejection, and timeout behavior.
2. Run verification-harness integration coverage to confirm readiness-related
   outcomes remain distinct from behavioral CoreOps failures.
3. When environment access is available, run VM-backed validation to confirm a
   healthy guest becomes reachable through the readiness record path without
   relying primarily on ARP-derived address discovery.

## Expected Operator Outcomes

- A healthy VM-backed run becomes reachable using a validated current-run
  readiness record.
- Stale or malformed console readiness data does not unblock the run.
- Missing readiness ends with an explicit readiness-related failure outcome.
- Diagnostic artifacts let operators tell whether readiness succeeded, timed
  out, was rejected, or fell back before later guest-boundary work.
- `artifacts/readiness-evidence.json` captures the accepted record, rejected
  records, final readiness status, and failure summary for the run.

## Out Of Scope

- General-purpose guest agent framework
- Dual-stack readiness contract
- Permanent parallel support for ARP as a first-class peer discovery mechanism
- Replacing later SSH readiness or behavioral checks once the guest address has
  been established
