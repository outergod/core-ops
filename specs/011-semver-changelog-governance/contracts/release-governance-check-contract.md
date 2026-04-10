# Contract: Release Governance Check

## Purpose

Define the observable behavior of the required pull request validation that
enforces SemVer and changelog governance.

## Inputs

- changed repository content in the pull request
- canonical version in `Cargo.toml`
- checked-in release fragment content
- generated or source changelog state
- release classification and SemVer decision rules

## Outcomes

### Passed

The check passes only when:

- the pull request is exempt under explicit classification rules, or
- the pull request is releasable and:
  - `Cargo.toml` carries the required version bump
  - required release fragment content is present
  - generated changelog content can remain aligned with the fragment
  - the declared bump matches the required bump

### Failed

The check fails when:

- releasable changes are present without required release metadata
- the declared SemVer impact is lower or otherwise different than required
- metadata-only changes are present without explicit release-preparation intent
- exempt changes are incorrectly used to mask releasable deltas

## Failure Reporting

A failed result MUST identify:

- whether the pull request was evaluated as exempt or releasable
- the effective required bump, if any
- which required artifacts are missing or inconsistent
- which policy rule caused the failure

## Branch Protection Use

The check is designed to serve as a required status check for pull requests
targeting the default branch.
