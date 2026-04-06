# Contract: Scenario Schema

## Purpose

Defines the conservative public contract for declarative verification scenarios
consumed by the CoreOps verification harness.

## Authoring Model

The schema separates three concerns:

- behavioral intent
- environment profile selection
- harness-policy overrides

Common-case scenarios should remain authorable on one screen by inheriting
standard profiles and omitted defaults. Inline overrides exist for intentional
deviations, not to restate routine harness configuration in every scenario.

## Schema Shape

```yaml
scenario_id: verify-idempotent-frontend
title: Frontend idempotency remains stable
description: Reapplying the same desired state produces no pending changes.
scenario_classes:
  - idempotency
source: accepted
behavioral_claim: Reapplying the same revision produces no managed changes.
rationale: Guards regression in convergence semantics.
environment:
  profile: single-blessed-vm
fixtures:
  repo_fixture: fixtures/repos/frontend
  revision_under_test: demo-uat-v2
steps:
  - step_id: boot
    step_type: boot
    target: guest
  - step_id: apply
    step_type: coreops_action
    target: guest
    action:
      action: apply
      repository_source: fixture
      revision: demo-uat-v2
  - step_id: reapply
    step_type: coreops_action
    target: guest
    action:
      action: apply
      repository_source: fixture
      revision: demo-uat-v2
assertions:
  - assertion_id: no-pending-change
    assertion_type: no_pending_changes
    target: guest
    expected_state: none
    failure_message: Reapply reported managed changes.
```

## Optional Overrides

The common case should omit routine harness configuration. Scenarios may add
explicit overrides only when they intentionally differ from profile defaults.

```yaml
environment:
  profile: single-blessed-vm
  overrides:
    image_version: "2026-04-15"

policy_overrides:
  timeout_profile: standard
  timeouts:
    scenario_timeout: 1800s
```

Raw command strings remain available for guest-command or explicit escape-hatch
steps, but common CoreOps operations should prefer structured semantic actions.

## Contract Rules

- `scenario_id` MUST be stable and unique within the accepted corpus.
- `scenario_classes` MUST contain one or more supported taxonomy values.
- `source` MUST be `accepted` or `candidate`.
- `environment.profile` MUST resolve to an approved environment profile.
- v1 environment profiles MUST declare exactly one guest.
- `steps` MUST contain only supported v1 step types:
  - `boot`
  - `wait_ready`
  - `coreops_action`
  - `guest_command`
  - `mutate_state`
  - `reboot`
- Structured semantic actions SHOULD be used for common CoreOps operations;
  raw command strings SHOULD be reserved for explicit guest commands or
  uncommon escape-hatch behavior.
- `assertions` MUST contain at least one assertion and each assertion MUST map
  to a stable contract rather than incidental log output.
- v1 supported assertion types are:
  - `no_pending_changes`
  - `output_contains`
  - `step_command_contains`
  - `step_command_not_contains`
  - `step_stdout_contains`
  - `step_exit_code_is`
  - `step_duration_within_ms`
- Timeout and artifact-retention policy SHOULD inherit from named defaults
  unless a scenario explicitly overrides them.
- Omitted optional collections such as empty config/test-data lists are
  preferred over schema noise in authored scenarios.

## Validation Failures

Scenario validation MUST reject documents that:

- lack assertions
- duplicate accepted coverage without new behavioral value
- rely on unstable signals
- require unsupported infrastructure
- exceed v1 single-VM scope
- omit behavioral claim or rationale
- attempt to restate invalid or unsupported environment/profile combinations

## Compatibility Notes

- This schema is a public authoring contract and MUST evolve conservatively.
- Additive optional fields, named profiles, and omitted-default inheritance are
  preferred over breaking field renames.
- Any externally visible schema change requires release-version review.
