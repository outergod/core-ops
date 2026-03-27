# CoreOps Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-03-27

## Active Technologies
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

- Rust (stable toolchain) + Git (CLI), systemd (systemctl), Podman/Quadlet generator (001-gitops-quadlet-controller)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust (stable toolchain): Follow standard conventions

## Recent Changes
- 007-explainable-reconcile-interface: Added Rust 2021 (`core-ops` 0.5.0) + `clap`, `miette`, `thiserror`, `serde`, `serde_json`, `log`, `systemd-journal-logger`, `tempfile`
- 006-deterministic-reconcile: Added Rust (stable toolchain, edition 2021) + Existing CoreOps Rust stack: clap, thiserror, miette, log, systemd-journal-logger, serde, serde_json, tempfile; systemd and Quadlet remain the runtime integration surfaces
- 005-native-mount-management: Added Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json, systemd native unit generation via existing CoreOps boundaries

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
