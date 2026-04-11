# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses semantic
versioning for public release policy decisions.

## [Unreleased]

<!-- core-ops-release:start -->
### Changed

- Add the core-ops-release helper binary for machine-checkable SemVer and changelog governance
- Unified CI validation and release publication into a single ci.yml workflow
<!-- core-ops-release:end -->

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
