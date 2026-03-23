# core-ops Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-03-20

## Active Technologies
- Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger (002-systemd-agent)
- Files on disk (Quadlet unit files + optional reconciliation state) (002-systemd-agent)
- Files on disk (repository layout + evaluated desired state in memory) (003-layered-overrides)

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
- 003-layered-overrides: Added Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger
- 002-systemd-agent: Added Rust (stable toolchain) + Git (CLI), systemd (systemctl), Quadlet generator, clap, thiserror, miette, journald logger
- 001-gitops-quadlet-controller: Added Rust (stable toolchain) + Git (CLI), systemd (systemctl), Podman/Quadlet generator

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
