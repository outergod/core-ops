# Contract: CLI Interface

**Audience**: Operators managing a single Fedora CoreOS host

## Commands

### Plan

- **Purpose**: Compute and display a reconciliation plan without applying changes.
- **Inputs**: repository reference, target revision (optional), mode=plan
- **Outputs**: plan summary, diffs, safety checks, and expected outcomes
- **Failure cases**: invalid desired state, repo unavailable, unsupported
  mutations, invariant violations

### Apply

- **Purpose**: Apply the reconciliation plan to converge host state.
- **Inputs**: repository reference, target revision (optional), mode=apply,
  explicit operator intent acknowledgement
- **Outputs**: actions applied, verification results, final outcome status
- **Failure cases**: validation/plan failures, apply failures, verification
  failures; all must be reported with failure classification

### Status

- **Purpose**: Report the last reconciliation outcome and current divergence.
- **Inputs**: none or repository reference
- **Outputs**: last run status, current diffs (if any), last known revision

### Validate

- **Purpose**: Validate desired state without planning or applying changes.
- **Inputs**: repository reference, target revision (optional)
- **Outputs**: validation results, boundary/invariant checks

## Output Contract

- All commands MUST emit a human-readable summary and a machine-readable form.
- All failures MUST include a failure class and a short recovery suggestion.

## Safety Contract

- Apply operations MUST require explicit operator intent.
- Plan and validate MUST not change host state.
