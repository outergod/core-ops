# core-ops Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-11

## Active Technologies

- Rust 2021 — clap 4, serde, miette, serde_json, serde_yaml
- GitHub Actions — ubuntu-latest runners, `gh` CLI, `rustup`

## Project Structure

```text
src/                          # Rust source (core-ops, core-ops-verify, core-ops-release)
tests/integration/            # Distribution and behavioral integration tests
.github/workflows/            # CI (ci.yml) and E2E gate (e2e-gate.yml)
specs/                        # Feature specifications and plans
changes/                      # Release intent fragments
```

## Commands

```bash
cargo build --locked --bin core-ops --bin core-ops-verify --bin core-ops-release
cargo test
cargo clippy --all-targets -- -D warnings
cargo run --bin core-ops-release -- validate --base-ref HEAD^
```

## Code Style

Follow standard Rust conventions. No new abstractions without justification.

## Recent Changes

- 012-unify-ci-release: Unified CI workflow — build matrix, artifact upload, release job

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
