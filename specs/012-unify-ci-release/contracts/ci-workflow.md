# Contract: Unified CI Workflow (ci.yml)

This contract describes the observable, machine-verifiable behavior of the
unified `.github/workflows/ci.yml`. Integration tests (`test_distribution_*`)
assert these invariants directly against the workflow file content.

---

## Trigger Contract

| Trigger | Jobs that run |
|---------|--------------|
| `pull_request` | `ci`, `build` (both targets) |
| `push` to `refs/heads/master` | `ci`, `build` (both targets), `release` |
| `push` to any other ref | `ci`, `build` (both targets) |

The `release` job MUST include an explicit `if` condition:
```
github.ref == 'refs/heads/master' && github.event_name == 'push'
```

---

## Required Workflow Snippets (asserted by integration tests)

### `ci` job — always present in `ci.yml`

```
cargo build --locked
cargo test
cargo clippy --all-targets -- -D warnings
core-ops-release -- validate --base-ref HEAD^
```

### `build` job — cross-compilation matrix

```
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
rustup target add
gcc-aarch64-linux-gnu
binutils-aarch64-linux-gnu
libc6-dev-arm64-cross
artifact_arch="amd64"
artifact_arch="arm64"
cp specs/002-systemd-agent/contracts/systemd/core-ops.service dist/core-ops.service
cp specs/002-systemd-agent/contracts/systemd/core-ops.timer dist/core-ops.timer
cp LICENSE dist/LICENSE
cp CHANGELOG.md dist/CHANGELOG.md
cp README.md dist/README.md
core-ops.service core-ops.timer LICENSE CHANGELOG.md README.md
release-metadata.json
dist/SHA256SUMS-${artifact_arch}
"release_gate_status": "passed"
"accepted_verification_status": "passed"
"verification_environment": "fedora-coreos-self-hosted@2026-04-fcos"
grep '^version' Cargo.toml
```

The workflow MUST NOT contain:
```
cp tests/fixtures/distribution/release-metadata.json dist/release-metadata.json
```
(design-contract fixture must not be shipped as release metadata)

### `release` job — master-push gate

```
refs/heads/master
git ls-remote --tags origin
gh release create
contents: write
```

---

## Artifact Name Contract

Workflow artifacts produced by the `build` job:
- `core-ops-binary-release-x86_64-unknown-linux-gnu`
- `core-ops-binary-release-aarch64-unknown-linux-gnu`

These names are stable and referenced by the `release` job's download step.

---

## Retired Workflow Contract

`release-binary.yml` MUST NOT exist in `.github/workflows/` once this feature
is complete (FR-007, SC-005). Any integration test that previously read
`release-binary.yml` MUST be updated to read `ci.yml` instead.

---

## README Credibility Badge Contract

The README `## Credibility` section MUST contain at minimum:

| Signal | Badge source |
|--------|-------------|
| CI status | `https://github.com/outergod/core-ops/actions/workflows/ci.yml/badge.svg` |
| E2E gate status | `https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml/badge.svg` |
| Latest release version | `https://img.shields.io/github/v/release/outergod/core-ops` (or equivalent) |

All three badges MUST be live-sourced (not static strings). Static descriptive
rows MAY remain alongside live badges where no live-data equivalent exists.
