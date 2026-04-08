# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project uses semantic
versioning for public release policy decisions.

## [Unreleased]

### Added

- Distribution-readiness planning and public outside-consumption contracts
- Serial-console readiness and accepted verification corpus expansion

### Changed

- Public release identity, version visibility, and verification expectations
  are being formalized for outside consumption

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
