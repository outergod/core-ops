# Quickstart: SemVer and Changelog Governance

## Goal

Validate the release-governance feature from the perspective of a maintainer or
agent authoring a releasable pull request.

## 1. Create a releasable change

- Modify a releasable file such as Rust source, accepted verification corpus,
  or a release/verification workflow with operator-facing impact.

## 2. Add release metadata

- Bump the canonical version in `Cargo.toml`
- Add or update the checked-in release fragment for the change
- Ensure the fragment declares `patch`, `minor`, or `major`

## 3. Validate locally

Run:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Then run the release-governance validation path once it exists to confirm:

```bash
cargo run --bin core-ops-release -- validate
```

Then confirm:

- releasable changes are detected
- the required bump is reported correctly
- missing metadata is rejected

## 4. Verify exempt behavior

- Create a docs-only or formatting-only change set
- Confirm the governance check treats it as exempt and does not require release
  metadata

## 5. Verify mismatch behavior

- Create a releasable change that should be `minor`
- Declare `patch` in the fragment
- Confirm the governance check fails and reports the required bump

## 6. Verify release-preparation behavior

- Create a metadata-only change
- Confirm it fails without explicit release-preparation designation
- Confirm it passes only when intentionally marked as release-preparation work
