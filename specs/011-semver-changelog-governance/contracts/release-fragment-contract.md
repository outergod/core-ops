# Contract: Release Fragment

## Purpose

Define the required semantic content for a checked-in release fragment used to
declare SemVer intent and generate `CHANGELOG.md`.

## Audience

- Maintainers reviewing releasable pull requests
- Contributors and agents authoring releasable changes
- CI logic validating release completeness

## Required Semantics

Each releasable pull request MUST contribute release fragment content that
provides:

- a machine-checkable SemVer intent: `patch`, `minor`, or `major`
- human-readable release-note text suitable for generated changelog content
- enough identity to associate the fragment with a single change set
- an explicit `release_preparation: true` designation when metadata-only
  changes are intentional

Each fragment is stored at `changes/<change-id>.md`.

## Validation Rules

- Fragments are required for every releasable pull request.
- Exempt pull requests MUST NOT be forced to add fragments.
- A fragment with invalid or missing SemVer intent fails governance validation.
- Metadata-only changes fail validation unless the fragment explicitly marks the
  pull request as release-preparation work with `release_preparation: true`.
- Accepted verification corpus changes require at least `patch`.

## Generated Changelog Relationship

- `CHANGELOG.md` is generated from approved fragment content.
- Fragment content therefore acts as the per-PR source of truth for release
  notes.
- Direct manual changelog edits are not the required per-PR contract.
