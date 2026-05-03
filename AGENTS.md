# CoreOps Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-10

## Active Technologies
- Rust (stable toolchain) + Git (CLI), systemd (systemctl), Podman/Quadlet generator (001-gitops-quadlet-controller)
- Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger (002-systemd-agent)
- Files on disk (Quadlet unit files + optional reconciliation state) (002-systemd-agent)
- Files on disk (repository layout + evaluated desired state in memory) (003-layered-overrides)
- Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json for the canonical status snapshot (004-reconcile-provenance)
- Files on disk under a runtime state directory for canonical persisted provenance; optional repository cache remains separate and non-authoritative (004-reconcile-provenance)
- Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json, systemd native unit generation via existing CoreOps boundaries (005-native-mount-management)
- Files on disk for desired-state repository content, generated native unit files, and existing canonical status state under `/var/lib/core-ops/status.json` (005-native-mount-management)
- Rust (stable toolchain, edition 2021) + Existing CoreOps Rust stack: clap, thiserror, miette, log, systemd-journal-logger, serde, serde_json, tempfile; systemd and Quadlet remain the runtime integration surfaces (006-deterministic-reconcile)
- Files on disk for desired-state repository content and the canonical persisted CoreOps status or reconciliation snapshot state under the runtime state directory (currently centered on `/var/lib/core-ops/status.json`) with bounded retained successful snapshots for rollback eligibility (006-deterministic-reconcile)
- Rust 2021 (`core-ops` 0.5.0) + `clap`, `miette`, `thiserror`, `serde`, `serde_json`, `log`, `systemd-journal-logger`, `tempfile` (007-explainable-reconcile-interface)
- Files on disk for persisted provenance and deterministic reconciliation state under the runtime state directory; machine-readable interface payloads are emitted transiently by CLI/report surfaces (007-explainable-reconcile-interface)
- Rust 2021 + Existing CoreOps Rust stack (`clap`, `miette`, (008-e2e-verification-harness)
- Files on disk for scenario definitions, accepted scenario corpus, (008-e2e-verification-harness)
- Files on disk for rendered ignition inputs, serial console logs, (009-serial-console-readiness)
- Rust 2021 for shipped binaries and verification tooling; Markdown/YAML/shell for public docs and automation definitions + Existing CoreOps Rust stack (`clap`, `miette`, `thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `systemd-journal-logger`, `tempfile`, `time`), git metadata from the repository, curl-consumable binary distribution surfaces, forge-hosted automation definitions (010-distribution-readiness)
- Files on disk for public documentation, changelog, license/code-of-conduct documents, workflow definitions, release metadata, and existing verification artifacts (010-distribution-readiness)
- Rust 2021 (stable toolchain) for validation logic; Markdown/YAML and GitHub Actions workflow definitions for repo-facing governance surfaces + Existing CoreOps Rust stack (`clap`, `serde_json`, `serde_yaml`, `miette`, `thiserror`, `tempfile`) plus standard library filesystem and process facilities; no new dependency is assumed in Phase 0/1 (011-semver-changelog-governance)
- Files on disk in the repository (`Cargo.toml`, `CHANGELOG.md`, release fragment files, workflow/test fixtures, specification artifacts) (011-semver-changelog-governance)

## Project Structure

```text
src/
tests/
```

## Commands

- `cargo test`
- `cargo clippy --all-targets -- -D warnings`

## Release Discipline

- When work changes externally visible behavior, contracts, release materials,
  support boundaries, or compatibility, agents must update `CHANGELOG.md` in
  Keep a Changelog format before considering the work complete.
- Releasable work must also update the canonical package version in
  `Cargo.toml` and the repository's machine-checkable release-intent artifact
  at `changes/<change-id>.md`.
- Agents must choose the SemVer bump (`patch`, `minor`, or `major`) according
  to the highest-impact change in the work set and treat PRs missing version,
  changelog, or release-intent updates as incomplete unless an explicit
  machine-checkable exemption applies.
- Intentional metadata-only release preparation must be declared with
  `release_preparation: true` in the checked-in release fragment.

### Release governance commands

The `core-ops-release` binary has three subcommands:

```bash
# Preview what CHANGELOG.md [Unreleased] will look like from current fragments
cargo run --bin core-ops-release -- changelog

# Validate the change set before committing or opening a PR
cargo run --bin core-ops-release -- validate --base-ref HEAD^

# Promote [Unreleased] → [<version>] and delete consumed fragments.
# CI runs this automatically on master push; humans rarely need it locally.
cargo run --bin core-ops-release -- promote --version <X.Y.Z>
```

`validate` checks that a fragment exists, `release_intent` is ≥ the inferred bump,
and `CHANGELOG.md` matches the generated output. `promote` is idempotent and is the
single source of truth for the `[Unreleased]` → `[<version>]` transition.

### CHANGELOG.md is machine-managed

The block between `<!-- core-ops-release:start -->` and `<!-- core-ops-release:end -->`
is owned by `core-ops-release`. **Never edit it by hand.** Render with
`cargo run --bin core-ops-release -- changelog --write`. The post-merge promote
step in CI is what moves rendered content into a `[<version>]` section — the
maintainer never does that step manually.

### Fragment lifecycle

| Stage | Action |
|---|---|
| Start releasable work | Create `changes/<feature-id>.md` |
| PR open / CI running | Keep the fragment |
| After master push | *Automatic — CI's promote step deletes the fragment* |

Do not delete fragments by hand. The post-merge `core-ops-release promote` step
in `.github/workflows/ci.yml` removes every consumed fragment under `changes/`
(except `README.md`) when it cuts the new `[<version>]` section. A fragment
left behind from a partial prior run is swept on the next master push.

### Bump rules and breaking changes

The governance model infers a minimum bump from file-change types:
`src/` modified → `patch`, `src/` added → `minor`, `src/` deleted/renamed → `major`.
It cannot detect breaking changes inside modified files (e.g. removing a CLI flag).
Declare `major` in the fragment whenever a public CLI surface is removed or renamed —
declaring higher than inferred is allowed.

## Code Style

Rust (stable toolchain): Follow standard conventions

## Recent Changes
- 016-source-repository-layout: Formalized on-disk source-repository shape around payload-kind subdirectories (`quadlet/`, `systemd/`, `config/`) at each service root; optional `service.yaml` declaring `config-root` for variant services; host overlays mirror service shape directly under `hosts/<host-id>/<svc-id>/`. Legacy parser (`quadlets/`, `quadlet-overrides/`, `hosts/<h>/overrides/`) removed. New `core-ops skill install` subcommand emits an agentskills.io-standard bundle to `.agents/skills/core-ops-source-repo/`. Four in-tree example shapes plus `scripts/migrate-legacy-source-repo.sh` for one-pass conversion. Major version bump 1.0.0 → 2.0.0.
- 011-semver-changelog-governance: Added Rust 2021 (stable toolchain) for validation logic; Markdown/YAML and GitHub Actions workflow definitions for repo-facing governance surfaces + Existing CoreOps Rust stack (`clap`, `serde_json`, `serde_yaml`, `miette`, `thiserror`, `tempfile`) plus standard library filesystem and process facilities; no new dependency is assumed in Phase 0/1
- 010-distribution-readiness: Added Rust 2021 for shipped binaries and verification tooling; Markdown/YAML/shell for public docs and automation definitions + Existing CoreOps Rust stack (`clap`, `miette`, `thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `systemd-journal-logger`, `tempfile`, `time`), git metadata from the repository, curl-consumable binary distribution surfaces, forge-hosted automation definitions
- 009-serial-console-readiness: Added Rust 2021 + Existing CoreOps Rust stack (`clap`, `miette`,

<!-- MANUAL ADDITIONS START -->
## Verification Harness Playbook

When working on feature or regression changes in this repository, agents MUST
treat VM-backed `core-ops-verify run` execution as the authoritative
verification path for end-to-end scenario coverage. Use `cargo test` only for
deterministic internal unit/integration coverage — it is not a substitute for
live host validation.

The detailed contracts and examples currently live under
`specs/008-e2e-verification-harness/`, but the scenario-generation and
regression-verification workflow is now a repo-wide development practice, not
just a one-off feature artifact.

### Mandatory E2E Scenario Authoring

E2E scenario authoring is **not optional**. Every accepted feature and every
regression fix must have a corresponding accepted scenario before the work is
considered complete.

**For every new feature (non-negotiable)**:
1. Author a candidate scenario covering the feature's primary behavioral claim
2. Place it in `tests/fixtures/verification/scenarios/` with
   `source: accepted` once the claim is stable
3. Scenario class must match the feature type: `convergence`, `idempotency`,
   `upgrade_transition`, etc.
4. The scenario is the live-host proof that `cargo test` cannot provide

**For every regression fix (non-negotiable)**:
1. Create a repository-history fixture under
   `tests/fixtures/verification/repos/<fix-name>-history/` with at least two
   revisions: the pre-fix state and the post-fix state
2. The fixture must use the layered repo format (`services/` + `hosts/`) if it
   involves config files — the simple `quadlets/` format does not support
   `managed_config_paths`
3. Author an accepted scenario with `scenario_classes: [regression_detection]`
   referencing `fixtures.repository_evolution`
4. The scenario's key assertion must directly test the behavioral claim of the
   fix (e.g., "1 restart" for a planner restart fix)
5. Preserve the scenario in the accepted corpus permanently — it is the
   permanent regression guard

**Workflow for regression scenarios**:
```
1. Create repo fixture: tests/fixtures/verification/repos/<name>-history/
     <revision-1>/  — state that exposed the bug
     <revision-2>/  — state after the fix applies correctly
2. Author scenario YAML in tests/fixtures/verification/scenarios/
     source: accepted
     scenario_classes: [regression_detection]
     fixtures.repository_evolution.history_fixture: <path>
     assertions: include the specific behavioral check (e.g., "1 restart")
3. Validate scenario parses: cargo test --test mod verification_cli
4. Run on live VM: core-ops-verify run --scenario <path>
5. Mark the implementation task complete only after step 4 passes
```

**Layered repo format** (required for config-file scenarios):
```
<revision>/
  services/<name>/<name>.container        ← container quadlet with EnvironmentFile=
  services/<name>/config/etc/<path>       ← config file (target: /etc/<path>)
  hosts/<hostname>/host.yaml              ← host: <hostname>\nservices:\n  - <name>
```
Use `action.host: <hostname>` in the scenario step to pass `--host <hostname>`
to `core-ops apply`, matching the hostname in `host.yaml`.

### Scenario / Bundle Generation Rules

- **Testing an existing accepted feature**:
  - Prefer rerunning an accepted scenario from
    `tests/fixtures/verification/scenarios/`
  - Use:
    - `core-ops-verify run --scenario <accepted-scenario>`
    - or `core-ops-verify run --accepted-dir tests/fixtures/verification/scenarios --ci`
- **Testing a new feature before acceptance**:
  - Generate or author a candidate under
    `tests/fixtures/verification/generated_candidates/`
  - Review the candidate for stable behavioral claims, supported taxonomy, and
    durable assertions before promoting it into the accepted corpus
- **Testing a regression or real bug reproduction**:
  - Create or extend a repository-history fixture under
    `tests/fixtures/verification/repos/`
  - Author or promote an accepted scenario that references
    `fixtures.repository_evolution`
  - Keep accepted regression scenarios in
    `tests/fixtures/verification/scenarios/` permanently after the fix is
    validated

### Agent Workflow By Use Case

1. **Existing accepted scenario rerun**
   - Find the accepted scenario in
     `tests/fixtures/verification/scenarios/`
   - Run it directly
   - Inspect the artifact bundle and report output

2. **New feature coverage**
   - Start from the feature specification
   - Generate a candidate with `core-ops-verify generate`
   - Review and normalize the candidate
   - Promote it into `tests/fixtures/verification/scenarios/` only after the
     scenario adds stable coverage

3. **Regression reproduction and promotion**
   - Encode the pre-fix and post-fix revision sequence in a repository-history
     fixture
   - Add a regression-detection scenario that references that history
   - Validate the failure before the fix and the pass after the fix
   - Preserve the accepted scenario as a permanent regression entry

### Expected Bundle Outputs

- Every verification run should retain:
  - scenario definition
  - harness log
  - console log
  - CoreOps command output
  - assertion results
- Failed regression-oriented runs should also retain:
  - `failure-summary.txt`
  - `regression-summary.txt`
  - `promotion-status.txt` for accepted regression scenarios

### Operational Notes

- If no libvirt override is set, verification uses local libvirt
  (`qemu:///system`)
- Remote hypervisor selection uses `CORE_OPS_VERIFY_VM_HOST` or
  `CORE_OPS_VERIFY_LIBVIRT_URI`
- VM-backed runs normally also require
  `CORE_OPS_VERIFY_CORE_OPS_BIN=target/debug/core-ops`
<!-- MANUAL ADDITIONS END -->
