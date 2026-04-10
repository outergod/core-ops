# Release Governance Fixtures

This directory contains fixture repositories and files used by release-governance
unit and integration tests.

Fixture naming guidance:

- `releasable-*` for changes that must require release metadata
- `exempt-*` for changes that must pass without release metadata
- `mixed-*` for change sets containing both releasable and exempt deltas
- `metadata-only-*` for version, changelog, or fragment-only scenarios
