# Data Model: Serial Console Readiness

## ReadinessRecord

- **Purpose**: Structured guest-published readiness message consumed by the
  VM-backed verification harness.
- **Fields**:
  - `run_id`: stable identifier for the current verification run
  - `token`: run-scoped secret or nonce used to reject stale records
  - `ip`: authoritative guest IPv4 address for later guest-boundary steps
  - `hostname`: optional guest hostname for diagnostics
  - `ts`: optional guest-reported timestamp for diagnostics
- **Validation rules**:
  - `run_id`, `token`, and `ip` are required
  - `ip` must parse as a usable IPv4 address
  - Unknown additional fields must not invalidate the core required fields if
    additive evolution is allowed later

## ReadinessExpectation

- **Purpose**: Run-scoped values the harness expects before it accepts a
  readiness record.
- **Fields**:
  - `run_id`
  - `token`
  - `deadline`
  - `fallback_policy`
- **Validation rules**:
  - `deadline` must be derived from the configured readiness timeout
  - `fallback_policy` must preserve serial-console readiness as primary

## ReadinessAcquisitionState

- **Purpose**: Internal run-tracking state for readiness acquisition.
- **States**:
  - `waiting`
  - `accepted`
  - `rejected_stale`
  - `rejected_malformed`
  - `timed_out`
  - `fallback_used`
- **Transition rules**:
  - `waiting -> accepted` on the first valid matching readiness record
  - `waiting -> rejected_stale` when a mismatched run id or token is observed
  - `waiting -> rejected_malformed` when a structured record is present but
    invalid
  - `waiting -> timed_out` when the readiness window expires before acceptance
  - `waiting -> fallback_used` only when migration fallback is allowed and no
    accepted readiness record has been found
  - `accepted` is terminal for readiness acquisition in a given run

## GuestReadinessPayload

- **Purpose**: Guest-side injected configuration required to emit the readiness
  record.
- **Fields**:
  - `run_id`
  - `token`
  - `console_marker`
  - `service_name`
  - `script_path`
- **Validation rules**:
  - The console marker must remain stable and unique enough for the host parser
    to identify candidate readiness lines
  - `run_id` and `token` must match the host-side `ReadinessExpectation`

## ReadinessEvidence

- **Purpose**: Artifact/report representation of what the harness observed while
  establishing readiness.
- **Fields**:
  - `source`: serial console or fallback path
  - `accepted_record`: optional `ReadinessRecord`
  - `rejected_records`: optional list of rejection summaries
  - `final_status`
  - `failure_summary`: optional concise explanation
- **Validation rules**:
  - `final_status` must align with the run outcome reported to operators
  - Accepted and rejected records must not contradict one another for the same
    run
