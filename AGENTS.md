# CoreOps Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-08

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

## Code Style

Rust (stable toolchain): Follow standard conventions

## Recent Changes
- 010-distribution-readiness: Added Rust 2021 for shipped binaries and verification tooling; Markdown/YAML/shell for public docs and automation definitions + Existing CoreOps Rust stack (`clap`, `miette`, `thiserror`, `serde`, `serde_json`, `serde_yaml`, `log`, `systemd-journal-logger`, `tempfile`, `time`), git metadata from the repository, curl-consumable binary distribution surfaces, forge-hosted automation definitions
- 009-serial-console-readiness: Added Rust 2021 + Existing CoreOps Rust stack (`clap`, `miette`,
- 008-e2e-verification-harness: Added Rust 2021 + Existing CoreOps Rust stack (`clap`, `miette`,

<!-- MANUAL ADDITIONS START -->
## Verification Harness Playbook

When working on feature or regression changes in this repository, agents should
treat VM-backed `core-ops-verify run` execution as the authoritative
verification path for end-to-end scenario coverage. Use the synthetic path
only for deterministic internal test coverage.

The detailed contracts and examples currently live under
`specs/008-e2e-verification-harness/`, but the scenario-generation and
regression-verification workflow is now a repo-wide development practice, not
just a one-off feature artifact.

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
