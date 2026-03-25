# CoreOps Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-03-24

## Active Technologies
- Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger (002-systemd-agent)
- Files on disk (Quadlet unit files + optional reconciliation state) (002-systemd-agent)
- Files on disk (repository layout + evaluated desired state in memory) (003-layered-overrides)
- Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json for the canonical status snapshot (004-reconcile-provenance)
- Files on disk under a runtime state directory for canonical persisted provenance; optional repository cache remains separate and non-authoritative (004-reconcile-provenance)
- Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json, systemd native unit generation via existing CoreOps boundaries (005-native-mount-management)
- Files on disk for desired-state repository content, generated native unit files, and existing canonical status state under `/var/lib/core-ops/status.json` (005-native-mount-management)

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
- 005-native-mount-management: Added Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json, systemd native unit generation via existing CoreOps boundaries
- 004-reconcile-provenance: Added Rust (stable toolchain, edition 2021) + clap, thiserror, miette, log, systemd-journal-logger, tempfile, serde, serde_json for the canonical status snapshot; release version policy now tracks behavior/schema/CLI/reconciliation compatibility changes and uses `Cargo.toml` as the canonical controller version
- 003-layered-overrides: Added Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
