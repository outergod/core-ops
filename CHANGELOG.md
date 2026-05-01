# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses semantic
versioning for public release policy decisions.

## [Unreleased]

<!-- core-ops-release:start -->
### Changed

- Add controller lifecycle state management and core-ops init command
- Formalize source repository layout (payload-kind subdirs, optional service.yaml, host overlays mirroring service shape); remove legacy parser (quadlets/, quadlet-overrides/, hosts/<h>/overrides/); add `core-ops skill install` subcommand emitting an agentskills.io-standard bundle; ship four in-tree example shapes (minimal, variant-config-root, multi-unit, host-overlay) plus `scripts/migrate-legacy-source-repo.sh` for one-pass conversion of legacy repositories
<!-- core-ops-release:end -->

## [0.8.2] - 2026-04-12

### Fixed

- Config-file changes, removals, and additions (when the dependent container was already
  running) now correctly schedule `RestartUnit` actions for dependent containers during
  `apply`. Previously, the planner emitted only `WriteQuadlet` for `ConfigFile` diffs,
  leaving services silently running with stale configuration
- Apply report no longer falsely shows `restarted` for containers that were not actually
  restarted; a failed restart now correctly surfaces as `failed` in apply output

## [0.8.1] - 2026-04-12

### Fixed

- Short commit SHAs (e.g. `454ac5f1`) are now accepted as `--rev` input; previously
  the loader treated all revision inputs as fetchable refspecs, causing short and full
  SHAs to fail on most Git servers

## [0.8.0] - 2026-04-11

### Changed

- Add the `core-ops-release` helper binary for machine-checkable SemVer and changelog governance
- Unified CI validation and release publication into a single `ci.yml` workflow: build matrix
  produces cross-compiled artifacts (x86_64, aarch64) on every PR; release job on master push
  creates a GitHub Release with binary assets and CHANGELOG-sourced release notes
- Replace static README Credibility table values with live CI status, E2E gate status, and
  latest release version badges
- Retire `release-binary.yml`; `ci.yml` is now the sole workflow for both validation and publication

## [0.7.0] - 2026-04-11

### Changed

- Add the `core-ops-release` helper binary for machine-checkable SemVer and changelog governance

## [0.6.0] - 2026-04-07

### Added

- Dedicated VM-backed verification harness entrypoint
- Accepted verification scenarios for convergence, upgrade, reboot, timeout,
  and infrastructure-failure classes
- Machine-readable verification run output and readiness evidence artifacts

### Fixed

- Serial-console readiness parsing for prefixed console lines
- Retry behavior for transient console-read failures during readiness

## [0.5.0] - 2026-03-31

### Added

- Explainable reconcile interfaces
- Deterministic reconcile and retained state tracking

## [0.4.0] - 2026-03-26

### Added

- Canonical persisted provenance snapshots
- Mount-aware reconciliation semantics

### Fixed

- False-positive exit-code behavior around provenance and mount-management flows
- Automount handling issues discovered during native mount-management rollout

## [0.3.0] - 2026-03-24

### Added

- Canonical reconcile provenance and persisted status snapshots
- Machine-readable status output based on authoritative persisted state
- Release-policy and provenance traceability guidance for externally visible changes

## [0.2.0] - 2026-03-23

### Added

- Host-native systemd agent execution with canonical `core-ops.service` and
  `core-ops.timer` contracts
- Layered overrides for service selection and host-specific drop-ins
- Structured CLI surfaces for planning and applying against selected hosts

### Changed

- Unit and contract naming standardized around `core-ops` instead of
  `core-ops-agent`

## [0.1.0] - 2026-03-18

### Added

- Initial `core-ops` CLI scaffold and repository constitution
- GitOps-style Quadlet reconciliation foundation for systemd-based hosts
- Basic journald logging and temporary-workspace support for controller runs
