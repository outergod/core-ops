# core-ops Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-05-01

## Active Technologies
- Rust 2021 + clap 4, serde / serde_json, miette, thiserror, tempfile (015-controller-state-lifecycle)
- JSON state file at `/var/lib/core-ops/status.json` (atomic write via tempfile) (015-controller-state-lifecycle)
- Rust 2021 (stable toolchain), as established by the existing `core-ops` crate at v1.0.0; this feature is the trigger for the v2.0.0 major bump. + `clap` 4.5 (derive), `serde` 1.0 (derive), `serde_yaml` 0.9, `serde_json` 1.0, `miette` 7.2 (fancy diagnostics), `thiserror` 1.0, `tempfile` 3.10. No new runtime dependencies are required by this feature. (016-source-repository-layout)
- Source repository on filesystem (input); existing canonical status snapshot at `/var/lib/core-ops/status.json` (output). The status snapshot gains a `layout-version: "1"` field to record which layout produced it. (016-source-repository-layout)

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

The `core-ops-release` binary governs the release process. Subcommands:

```bash
# Preview what CHANGELOG.md will look like from current changes/ fragments
cargo run --bin core-ops-release -- changelog

# Validate that the current change set is releasable (run before every commit/PR)
cargo run --bin core-ops-release -- validate --base-ref HEAD^

# Promote [Unreleased] → [<version>] and delete the consumed fragments.
# CI runs this automatically on master push; humans rarely need it.
cargo run --bin core-ops-release -- promote --version <X.Y.Z>
```

**Fragment lifecycle (humans):**

1. **Create** `changes/<feature-id>.md` when starting a releasable change.
2. **Keep** it through the PR and until merge. The fragment populates the
   `[Unreleased]` block via `core-ops-release changelog --write`.
3. **Do not delete** the fragment by hand — the post-merge promote step
   in CI deletes it after the release is tagged.

**Release flow on master push (CI):**

1. `validate` re-checks the merged tree.
2. `promote --version <Cargo.toml>` moves the rendered `[Unreleased]`
   body into a new `## [<version>] - <date>` section, empties the
   `[Unreleased]` markers, and removes every consumed fragment under
   `changes/`. The promote commit is pushed back to master with
   `[skip ci]` so it doesn't loop.
3. `gh release create v<version>` publishes the GitHub Release at the
   merge SHA, which also creates the git tag.

`promote` is idempotent: if the tag already exists on origin the whole
release job short-circuits, and a stale fragment left behind from a
prior partial run is swept on the next master push.

**CHANGELOG.md is machine-managed** between `<!-- core-ops-release:start -->`
and `<!-- core-ops-release:end -->`. Never edit that section by hand.
For PR work, render with `core-ops-release changelog --write`.

**Declared vs required bump**: `release_intent` in the fragment must be ≥ the bump
inferred from file changes (`patch` for modified source, `minor` for added source,
`major` for deleted/renamed source). Declaring a higher intent than required is allowed
(e.g. `major` when only source was added) because the governance model cannot detect
breaking changes inside modified files. Use `major` whenever a public CLI surface is
removed or renamed.

## Code Style

Follow standard Rust conventions. No new abstractions without justification.

## Recent Changes
- 016-source-repository-layout: Added Rust 2021 (stable toolchain), as established by the existing `core-ops` crate at v1.0.0; this feature is the trigger for the v2.0.0 major bump. + `clap` 4.5 (derive), `serde` 1.0 (derive), `serde_yaml` 0.9, `serde_json` 1.0, `miette` 7.2 (fancy diagnostics), `thiserror` 1.0, `tempfile` 3.10. No new runtime dependencies are required by this feature.
- 015-controller-state-lifecycle: Added Rust 2021 + clap 4, serde / serde_json, miette, thiserror, tempfile

- 014-config-restart-fidelity: Fix planner to emit RestartUnit for config-file-dependent containers

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
