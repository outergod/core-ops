# Data Model: Unify CI Validation And Release Publication

## Workflow Job Graph

```
on: pull_request
  ci ──────────────────────────────────────── [terminal for PRs]
       └─► build[x86_64] ──► upload artifact
       └─► build[aarch64] ─► upload artifact

on: push (master)
  ci ──────────────────────────────────────── [gate]
       └─► build[x86_64] ──► upload artifact ──► release
       └─► build[aarch64] ─► upload artifact ──┘
```

## Job Definitions

### `ci` job

| Field | Value |
|-------|-------|
| Runs on | `ubuntu-latest` |
| Triggers | `pull_request`, `push` (master) |
| Permissions | `contents: read` |
| `fetch-depth` | `2` (for `HEAD^` governance check) |
| Outputs | None (test results only) |

Steps (in order):
1. `cargo build --locked --bin core-ops --bin core-ops-verify --bin core-ops-release`
2. `cargo test`
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo run --bin core-ops-release -- validate --base-ref HEAD^`

### `build` job (matrix)

| Field | Value |
|-------|-------|
| Runs on | `ubuntu-latest` |
| `needs` | `ci` |
| Triggers | Inherits (all triggers, but gated by `ci`) |
| Permissions | `contents: read` |
| Matrix | `target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]` |
| `fail-fast` | `false` |
| Env | `CORE_OPS_BUILD_SPEC_CONTEXT: "specs/010-distribution-readiness/spec.md"` |

Steps (in order):
1. `rustup target add "${{ matrix.target }}"`
2. (aarch64 only) Install cross-toolchain: `gcc-aarch64-linux-gnu`, `binutils-aarch64-linux-gnu`, `libc6-dev-arm64-cross`
3. `cargo build --release --locked --target "${{ matrix.target }}"`
4. Package artifacts into `dist/` (see Artifact Structure below)
5. `actions/upload-artifact@v4` → `name: core-ops-binary-release-${{ matrix.target }}`

### `release` job

| Field | Value |
|-------|-------|
| Runs on | `ubuntu-latest` |
| `needs` | `build` |
| `if` condition | `github.ref == 'refs/heads/master' && github.event_name == 'push'` |
| Permissions | `contents: write` |
| `fetch-depth` | `0` (full history for tag creation) |

Steps (in order):
1. Derive version: `grep '^version' Cargo.toml` → `version`, `tag = v${version}`
2. Check for duplicate tag: `git ls-remote --tags origin "refs/tags/${tag}"` → fail explicitly if found
3. Extract release notes: `awk` parse `CHANGELOG.md` block under `## [${version}]` → temp file
4. `actions/download-artifact@v4` with `merge-multiple: true` → `dist/`
5. `gh release create "${tag}" --title "core-ops ${tag}" --notes-file <temp> <assets...>`

## Artifact Structure (per matrix target)

Files produced into `dist/` by each `build` job matrix leg:

| File | Arch-specific? | Description |
|------|---------------|-------------|
| `core-ops-linux-amd64` / `core-ops-linux-arm64` | Yes | Raw release binary |
| `core-ops-linux-amd64.tar.gz` / `core-ops-linux-arm64.tar.gz` | Yes | Distribution tarball |
| `SHA256SUMS-amd64` / `SHA256SUMS-arm64` | Yes | Checksums for this arch |
| `core-ops.service` | No (identical) | systemd service unit |
| `core-ops.timer` | No (identical) | systemd timer unit |
| `LICENSE` | No (identical) | AGPL-3.0+ license |
| `CHANGELOG.md` | No (identical) | Changelog |
| `README.md` | No (identical) | README |
| `release-metadata.json` | No (identical) | Runtime release metadata |

**Tarball contents**: `core-ops-linux-${arch}`, `core-ops.service`, `core-ops.timer`, `LICENSE`, `CHANGELOG.md`, `README.md`

**Artifact names** (GitHub Actions):
- `core-ops-binary-release-x86_64-unknown-linux-gnu`
- `core-ops-binary-release-aarch64-unknown-linux-gnu`

## Release Assets Attached to GitHub Release

| Asset | Source |
|-------|--------|
| `core-ops-linux-amd64` | `dist/core-ops-linux-amd64` |
| `core-ops-linux-arm64` | `dist/core-ops-linux-arm64` |
| `core-ops-linux-amd64.tar.gz` | `dist/core-ops-linux-amd64.tar.gz` |
| `core-ops-linux-arm64.tar.gz` | `dist/core-ops-linux-arm64.tar.gz` |
| `SHA256SUMS-amd64` | `dist/SHA256SUMS-amd64` |
| `SHA256SUMS-arm64` | `dist/SHA256SUMS-arm64` |

## `release-metadata.json` Schema

```json
{
  "latest_release_identity": "v<version>",
  "release_gate_status": "passed",
  "accepted_verification_status": "passed",
  "artifact_availability": [
    "<arch> raw binary",
    "<arch> tar.gz + checksums",
    "systemd unit files",
    "LICENSE",
    "CHANGELOG.md",
    "README.md"
  ],
  "verification_environment": "fedora-coreos-self-hosted@2026-04-fcos",
  "credibility_location": "README.md#credibility"
}
```

`latest_release_identity` format: `v${version}` derived from `Cargo.toml` at build time.

## Permission Model

| Job | `contents` | Rationale |
|-----|-----------|-----------|
| `ci` | `read` | Read-only; no side effects |
| `build` | `read` | Read-only; artifacts are ephemeral |
| `release` | `write` | Required for tag creation and GitHub Release via `GITHUB_TOKEN` |

## Failure Modes

| Condition | Behavior |
|-----------|----------|
| Governance check fails | `ci` job fails; `build` and `release` are skipped |
| Binary build fails | `build` job fails; `release` is skipped |
| Duplicate version tag on master push | `release` job fails with explicit message; no tag is created |
| `gh release create` fails after tag is pushed (orphaned tag) | Subsequent runs fail duplicate-tag check; operator must manually delete tag to retry |
| Push to non-master branch | `release` job skipped via `if` condition |
