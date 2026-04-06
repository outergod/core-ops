# Quickstart: E2E Verification Harness with LLM-Assisted Scenario Generation

## Purpose

Show the intended development, release, and CI workflow for accepted scenario
execution, repository-evolution verification, candidate generation, and
offline diagnosis in v1.

VM-backed disposable-machine execution is the authoritative verification path
for this feature. Any synthetic or non-VM backend exists only to support
deterministic internal validation and does not satisfy the product goal by
itself.

## Prerequisites

- Linux host with access to the approved libvirt/KVM environment
- Approved guest image family available locally
- CoreOps build matching the revision under test
- Accepted scenario corpus available on disk

Runtime selection notes:

- if no libvirt override is set, `core-ops-verify run` uses local libvirt
  (`qemu:///system`)
- `CORE_OPS_VERIFY_VM_HOST=<host>` selects the common remote hypervisor path
- `CORE_OPS_VERIFY_LIBVIRT_URI=<uri>` fully overrides the libvirt connection
  target and takes precedence over `CORE_OPS_VERIFY_VM_HOST`
- VM-backed runs normally also need
  `CORE_OPS_VERIFY_CORE_OPS_BIN=target/debug/core-ops`

## Local Execution

1. Select an accepted scenario from the maintained corpus.
2. Start a local verification run through the dedicated verification tool
   entrypoint against a pinned revision under test or a repository-evolution
   fixture sequence.
3. Review the conclusive pass/fail result and inspect the retained artifact
   bundle.
4. If deeper investigation is needed, rerun in debug mode to retain the
   disposable environment for manual inspection.
5. For feature development or triage, rerun only the focused scenario subset
   needed to reproduce the bug or validate the change.

Example:

```bash
cargo run --bin core-ops-verify -- run \
  --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml \
  --json
```

Expected outcome:
- the run creates an isolated disposable workspace
- the scenario executes against one VM
- the scenario may exercise a realistic Git-history sequence with valid or
  invalid target states across multiple revisions
- the scenario normally references named environment and policy profiles rather
  than restating routine harness defaults inline
- the environment is torn down after artifact collection unless debug retention
  was explicitly requested

Important note:
- local verification should use the disposable-VM execution path; fast
  deterministic internal validation helpers may exist for development and test
  stability, but they are not a substitute for VM-backed E2E verification

## CI Gating

1. Run only accepted scenarios from the maintained corpus.
2. Emit machine-readable run results and deterministic exit status.
3. Export the retained artifact bundle for post-failure diagnosis.
4. Treat generated candidate scenarios as advisory until reviewed and accepted.

Example:

```bash
cargo run --bin core-ops-verify -- run \
  --accepted-dir tests/fixtures/verification/scenarios \
  --ci --json
```

Expected outcome:
- CI can gate on the run result without interactive interpretation
- failed runs remain diagnosable without rerunning immediately
- CI gating still represents disposable-VM verification rather than a purely
  synthetic harness result
- machine-readable output identifies the batch revision-selection basis and
  preserves per-scenario revision-under-test provenance

## Candidate Scenario Workflow

1. Provide a feature specification and any relevant behavioral contracts.
2. Generate candidate scenarios with declared behavioral claims, rationale, and
   taxonomy classification.
3. Validate and review generated candidates.
4. Accept only the scenarios that add stable, supported coverage.
5. Promote accepted scenarios, including bug reproductions, into the maintained
   corpus for local and CI use.

Authoring expectation:
- common scenarios should stay short by relying on named environment/policy
  profiles and semantic step actions
- scenario-local overrides should appear only when intentionally deviating from
  default harness behavior
- accepted bug reproductions should remain in the maintained corpus as
  permanent regression scenarios after the corresponding fix is validated

## Agent Workflow Matrix

Use the following playbook when deciding what to generate.

- **Existing accepted feature behavior**
  - rerun an accepted scenario from `tests/fixtures/verification/scenarios/`
  - inspect the retained bundle rather than generating a new candidate first
- **New feature without accepted coverage**
  - generate a candidate into
    `tests/fixtures/verification/generated_candidates/`
  - review and refine it
  - promote it into the accepted corpus only after the scenario proves stable
- **Regression or real bug reproduction**
  - encode the revision history under `tests/fixtures/verification/repos/`
  - author or promote an accepted `regression_detection` scenario that
    references `fixtures.repository_evolution`
  - preserve that scenario permanently after the fix is validated
- **Release or CI gating**
  - run only the accepted corpus
  - use focused reruns with repeated `--scenario-id` for triage

### Bundle Expectations By Use Case

- Existing-feature rerun:
  - expect the standard retained bundle and a conclusive pass/fail result
- New-feature candidate review:
  - expect reviewable scenario YAML plus later verification bundles after the
    scenario is accepted and run
- Regression-failure diagnosis:
  - expect standard retained artifacts plus:
    - `failure-summary.txt`
    - `regression-summary.txt`
    - `promotion-status.txt` for accepted regression scenarios

## Diagnostic Expectations

Every run retains:

- scenario definition
- harness log
- VM definition
- console output
- CoreOps command outputs
- assertion results

Failed runs additionally retain:

- relevant service state
- relevant files
- explain output
- revision identifiers

Scenarios may additionally assert:

- public command-surface behavior across supported human-readable,
  machine-readable, interactive, agent, and non-interactive interfaces
- coarse operational timing and responsiveness guardrails where meaningful

## Out of Scope for v1

- multi-VM scenarios
- multiple virtualization backends
- broad guest image matrix
- autonomous generated-scenario acceptance
- synthetic execution as a public replacement for VM-backed verification
