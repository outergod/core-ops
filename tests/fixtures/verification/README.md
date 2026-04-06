# Verification Fixtures

This directory holds declarative scenario fixtures, generated candidate
examples, and run-result artifacts for the feature-008 verification harness.

## Layout

- `scenarios/`
  Accepted scenarios used for local reruns and CI gating.
- `generated_candidates/`
  Advisory candidate scenarios produced from feature specifications before
  review and acceptance.
- `artifacts/`
  Contract fixtures for machine-readable verification run results.
- `repos/`
  Repository-history fixtures used to model realistic revision transitions,
  bug reproductions, and regression reruns.

## Authoring Conventions

Accepted scenarios should optimize for authorability in the common case.

- prefer named environment and policy profiles over repeating routine harness
  configuration inline
- use semantic CoreOps actions for common steps
- reserve inline overrides for deliberate deviations from the default profile
- keep scenario classes, behavioral claims, rationale, and structured
  assertions explicit

## Candidate Review and Acceptance

Generated candidates are advisory until reviewed and accepted into the
maintained corpus.

Typical flow:

1. generate a candidate from the feature specification
2. review its behavioral claim, scenario classes, and assertions
3. reject duplicates, unstable signals, or unsupported infrastructure
4. accept only candidates that add stable coverage
5. move the accepted scenario into `scenarios/`

## Agent Generation Matrix

Use this when deciding what to create.

- **Existing accepted feature**
  - do not create a new candidate by default
  - rerun the accepted scenario from `scenarios/`
  - inspect the retained bundle for diagnosis
- **New feature coverage**
  - generate or author a candidate in `generated_candidates/`
  - review its behavioral claim, taxonomy, and assertions
  - promote it into `scenarios/` only after it proves stable and valuable
- **Regression or bug reproduction**
  - create or update the repository-history fixture in `repos/`
  - add or promote an accepted scenario that references that history
  - keep the accepted regression scenario permanently after the fix is
    validated

## Expected Bundle Outputs

- standard verification bundles retain:
  - scenario definition
  - harness log
  - console output
  - CoreOps command outputs
  - assertion results
- failed regression-oriented runs should additionally retain:
  - `failure-summary.txt`
  - `regression-summary.txt`
  - `promotion-status.txt` for accepted regression scenarios

## Bug-Reproduction Promotion

Real bug reproductions should be turned into permanent accepted regression
scenarios once the fix exists.

Typical flow:

1. capture or author the repository-history fixture that reproduces the bug
2. add or accept a scenario that encodes the reproduction
3. validate that the scenario fails before the fix and passes after it
4. keep the accepted scenario in `scenarios/` as a permanent regression entry

## Developer Workflow Notes

- single-scenario runs use `core-ops-verify run --scenario <path>`
- accepted-corpus CI runs use
  `core-ops-verify run --accepted-dir tests/fixtures/verification/scenarios --ci`
- focused corpus reruns use repeated `--scenario-id <id>`
- the public verification path is VM-backed by default; synthetic execution is
  hidden internal support only
