# Data Model: E2E Verification Harness with LLM-Assisted Scenario Generation

## ScenarioDefinition

- **Purpose**: Declarative input describing one verification scenario.
- **Fields**:
  - `scenario_id`: Stable unique identifier within the accepted corpus
  - `title`: Human-readable short name
  - `description`: Behavioral purpose of the scenario
  - `scenario_classes`: One or more taxonomy classes
  - `source`: `accepted` or `candidate`
  - `behavioral_claim`: Explicit claim under test
  - `rationale`: Why the scenario exists
  - `environment`: `EnvironmentSelection`
  - `fixtures`: `FixtureSet`
  - `steps`: ordered list of `ScenarioStep`
  - `assertions`: ordered list of `AssertionSpec`
  - `policy_overrides`: optional `HarnessPolicyOverride`
- **Validation rules**:
  - Must contain at least one step and one assertion
  - Must declare at least one supported `scenario_class`
  - `source=accepted` requires prior review acceptance metadata
  - Common-case scenarios SHOULD rely on named profiles and omitted defaults
    rather than restating routine harness configuration inline

## EnvironmentSelection

- **Purpose**: Declares which standard runtime shape the scenario needs.
- **Fields**:
  - `profile`: `EnvironmentProfileId`
  - `overrides`: optional `EnvironmentOverride`
- **Validation rules**:
  - `profile` must resolve to an approved environment profile
  - v1 profiles must resolve to exactly one guest machine
  - Overrides may narrow or specialize the selected profile but may not violate
    approved backend/image constraints

## EnvironmentProfile

- **Purpose**: Reusable execution defaults for the common case.
- **Fields**:
  - `profile_id`
  - `backend_family`
  - `guest_image_family`
  - `image_version`
  - `network_policy`
  - `bootstrap_policy`
  - `guest`: `GuestProfile`
  - `default_policy`: optional `HarnessPolicyProfileId`
- **Validation rules**:
  - `backend_family` and `guest_image_family` must be from approved lists
  - Network policy must forbid implicit internet dependency
  - Exactly one guest is allowed in v1

## GuestProfile

- **Purpose**: Describes the standard VM shape under test.
- **Fields**:
  - `guest_name`
  - `cpu_profile`
  - `memory_profile`
  - `disk_overlay_policy`
  - `readiness_checks`
  - `connection_profile`
- **Validation rules**:
  - Must provide at least one explicit readiness condition
  - Resource identifiers must be deterministic from `run_id`

## EnvironmentOverride

- **Purpose**: Optional scenario-local deviations from the named environment
  profile.
- **Fields**:
  - `image_version`
  - `readiness_checks`
  - `connection_profile`
  - `resource_profile_overrides`
- **Validation rules**:
  - Omitted fields inherit from the selected profile
  - Overrides must remain compatible with the approved environment family

## HarnessPolicyProfile

- **Purpose**: Reusable timeout, artifact-retention, and debug-behavior
  defaults.
- **Fields**:
  - `profile_id`
  - `timeouts`: `TimeoutPolicy`
  - `artifact_policy`: `ArtifactPolicy`
- **Validation rules**:
  - Profiles must provide the required always-collected artifact baseline
  - Default execution may not retain the environment after artifact collection

## HarnessPolicyOverride

- **Purpose**: Optional scenario-local overrides to default harness policy.
- **Fields**:
  - `timeout_profile`: optional profile reference
  - `timeouts`: optional `TimeoutPolicy`
  - `artifact_profile`: optional profile reference
  - `artifact_policy`: optional `ArtifactPolicy`
- **Validation rules**:
  - Omitted fields inherit from the selected profile chain
  - Explicit overrides should be used only when the scenario intentionally
    deviates from the common path

## FixtureSet

- **Purpose**: Inputs staged before scenario execution.
- **Fields**:
  - `repo_fixture`
  - `repository_evolution`: optional `RepositoryEvolutionModel`
  - `config_inputs`: optional list
  - `test_data`: optional list
  - `revision_under_test`
- **Validation rules**:
  - Revision under test must be explicit for gating runs
  - Fixtures must reference pinned local inputs only
  - Empty optional collections should be omitted from authored scenarios

## RepositoryEvolutionModel

- **Purpose**: Describes the Git history and revision sequence exercised by the
  scenario.
- **Fields**:
  - `history_fixture`
  - `revisions`
  - `states`: valid or invalid target-state markers
  - `transition_expectations`
- **Validation rules**:
  - v1 must support authored fixture histories
  - Revisions must be ordered and refer to pinned local fixture inputs

## ScenarioStep

- **Purpose**: Ordered semantic action executed by the harness.
- **Fields**:
  - `step_id`
  - `step_type`: boot, wait_ready, coreops_action, guest_command, mutate_state,
    reboot
  - `target`: guest or harness context
  - `action`: optional structured `CoreOpsAction`
  - `command`: optional raw guest command or explicit escape hatch
  - `expected_exit_behavior`
  - `timeout_override`
- **Validation rules**:
  - Step type must be one of the supported v1 types
  - Common CoreOps operations should prefer structured semantic actions over
    raw CLI strings
  - Reboot and mutation steps require a subsequent readiness/assertion path
  - No step may rely on arbitrary sleep without an explicit readiness condition

## CoreOpsAction

- **Purpose**: Structured semantic representation of a common CoreOps command
  action.
- **Fields**:
  - `action`: apply, explain, plan, status, agent
  - `repository_source`
  - `revision`
  - `mode`
  - `output_contract`
- **Validation rules**:
  - Must map to documented public operational contract behavior
  - Must remain stable even if CLI spelling evolves

## AssertionSpec

- **Purpose**: Deterministic claim used to decide pass/fail.
- **Fields**:
  - `assertion_id`
  - `assertion_type`
  - `target`
  - `expected_state`
  - `failure_message`
  - `artifact_hints`
- **Validation rules**:
  - Must target stable contracts rather than incidental logs
  - Must be evaluable from collected observations

## TimeoutPolicy

- **Purpose**: Bounds scenario execution.
- **Fields**:
  - `per_step_defaults`
  - `scenario_timeout`
  - `readiness_timeout`
- **Validation rules**:
  - Scenario timeout must exceed all referenced step timeouts
  - Every waiting step must have an explicit timeout

## ArtifactPolicy

- **Purpose**: Declares what to retain and when.
- **Fields**:
  - `always_collect`
  - `collect_on_failure`
  - `retain_environment_in_debug`
  - `export_format`
- **Validation rules**:
  - Always-collected artifacts must include scenario definition, harness log,
    VM definition, console output, CoreOps outputs, and assertion results
  - Omitted fields inherit from the selected policy profile

## CandidateScenario

- **Purpose**: Advisory generated scenario proposal before acceptance.
- **Fields**:
  - `candidate_id`
  - `proposed_definition`: `ScenarioDefinition`
  - `generation_inputs`
  - `validation_findings`
  - `review_status`
- **State transitions**:
  - `generated -> rejected`
  - `generated -> needs_review`
  - `needs_review -> accepted`
  - `needs_review -> rejected`
- **Validation rules**:
  - Must include behavioral claim, rationale, and taxonomy classification
  - Rejected if malformed, redundant, unstable, unsupported, or purposeless

## VerificationRun

- **Purpose**: Single execution record for one or more scenarios.
- **Fields**:
  - `run_id`
  - `mode`: local, ci, debug
  - `revision_under_test`
  - `controller_version`
  - `scenario_refs`
  - `workspace_path`
  - `started_at`
  - `completed_at`
  - `overall_outcome`: `RunOutcome`
  - `artifact_bundle`: `ArtifactBundle`
- **Validation rules**:
  - `run_id` must be unique
  - `overall_outcome` must be derivable from scenario outcomes and failures

## ScenarioOutcome

- **Purpose**: Per-scenario execution result within a run.
- **Fields**:
  - `scenario_id`
  - `outcome`: passed, assertion_failure, infrastructure_failure, timeout,
    harness_error
  - `step_results`
  - `assertion_results`
  - `failure_summary`
- **Validation rules**:
  - Outcome classification must be mutually exclusive and explicit

## AssertionResult

- **Purpose**: Recorded evaluation of one assertion.
- **Fields**:
  - `assertion_id`
  - `status`: passed, failed, timed_out, not_evaluated
  - `observed_value`
  - `evidence_refs`
- **Validation rules**:
  - `not_evaluated` requires an explicit reason in associated step/run failure

## ArtifactBundle

- **Purpose**: Offline-diagnosable retained evidence from a run.
- **Fields**:
  - `bundle_path`
  - `manifest_entries`
  - `always_collected_entries`
  - `failure_specific_entries`
  - `environment_retained`
- **Validation rules**:
  - Must be sufficient to distinguish assertion failure, infrastructure
    failure, timeout, and harness error without rerun

## Relationships

- `ScenarioDefinition` has one `EnvironmentSelection`, one `FixtureSet`, many
  `ScenarioStep`, many `AssertionSpec`, and optional `HarnessPolicyOverride`.
- `EnvironmentSelection` resolves one `EnvironmentProfile` and may apply one
  `EnvironmentOverride`.
- `EnvironmentProfile` may reference one default `HarnessPolicyProfile`.
- `FixtureSet` may include one `RepositoryEvolutionModel`.
- `ScenarioStep` may embed one `CoreOpsAction`.
- `CandidateScenario` wraps one proposed `ScenarioDefinition`.
- `VerificationRun` contains many `ScenarioOutcome` and one `ArtifactBundle`.
- `ScenarioOutcome` contains many `AssertionResult`.
