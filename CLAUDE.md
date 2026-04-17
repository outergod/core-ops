# core-ops Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-15

## Active Technologies
- Rust 2021 + clap 4, serde / serde_json, miette, thiserror, tempfile (015-controller-state-lifecycle)
- JSON state file at `/var/lib/core-ops/status.json` (atomic write via tempfile) (015-controller-state-lifecycle)

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

## Release Governance

The `core-ops-release` binary governs the release process. Its two subcommands are:

```bash
# Preview what CHANGELOG.md will look like from current changes/ fragments
cargo run --bin core-ops-release -- changelog

# Validate that the current change set is releasable (run before every commit/PR)
cargo run --bin core-ops-release -- validate --base-ref HEAD^
```

**Fragment lifecycle — the one rule that prevents stale entries:**

1. **Create** `changes/<feature-id>.md` when starting a releasable change.
2. **Keep** it through the PR and until the release is tagged and published.
3. **Delete** it immediately after the release is tagged — before any further development.
   Fragments left behind will accumulate in the next `[Unreleased]` entry.

**CHANGELOG.md is machine-managed** between `<!-- core-ops-release:start -->` and
`<!-- core-ops-release:end -->`. Never edit that section by hand. Instead:

```bash
# Write the generated content into CHANGELOG.md
cargo run --bin core-ops-release -- changelog > /tmp/new-changelog.md
# Then copy the [Unreleased] block from /tmp/new-changelog.md into CHANGELOG.md
```

Or just let the release job do it — CI rewrites the section automatically on tag.

**Declared vs required bump**: `release_intent` in the fragment must be ≥ the bump
inferred from file changes (`patch` for modified source, `minor` for added source,
`major` for deleted/renamed source). Declaring a higher intent than required is allowed
(e.g. `major` when only source was added) because the governance model cannot detect
breaking changes inside modified files. Use `major` whenever a public CLI surface is
removed or renamed.

## Code Style

Follow standard Rust conventions. No new abstractions without justification.

## Recent Changes
- 015-controller-state-lifecycle: Added Rust 2021 + clap 4, serde / serde_json, miette, thiserror, tempfile

- 014-config-restart-fidelity: Fix planner to emit RestartUnit for config-file-dependent containers
- 012-unify-ci-release: Unified CI workflow — build matrix, artifact upload, release job

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
