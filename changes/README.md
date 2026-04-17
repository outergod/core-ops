# Release Fragments

CoreOps uses checked-in release fragments to declare machine-checkable release
intent for releasable pull requests.

Each releasable change set must add or update exactly one fragment at:

- `changes/<change-id>.md`

## Fragment format

```md
---
change_id: example-change
release_intent: patch
summary: Short human-readable release note
scope: optional-scope
release_preparation: false
---
```

Fields:

- `release_intent`: one of `patch`, `minor`, or `major` (see bump rules below)
- `release_preparation: true`: required only for intentional metadata-only changes
- `summary`: becomes the bullet in `CHANGELOG.md [Unreleased]`

## Fragment lifecycle

| Stage | Action |
|---|---|
| Start feature work | Create `changes/<feature-id>.md` |
| During PR review | Keep the fragment; CI validates it |
| After release is tagged | **Delete the fragment immediately** |

Fragments left in `changes/` after a release will appear in the _next_ `[Unreleased]`
entry and corrupt the changelog. Delete them as part of the release checklist, not
as a follow-up.

## Working with the changelog

`CHANGELOG.md` is machine-managed between the `<!-- core-ops-release:start -->` and
`<!-- core-ops-release:end -->` markers. **Never edit that section by hand.**

Use these commands instead:

```bash
# Preview the generated [Unreleased] block
cargo run --bin core-ops-release -- changelog

# Validate the current change set is releasable
cargo run --bin core-ops-release -- validate --base-ref HEAD^
```

The `validate` command checks that:
- A fragment exists for the current change
- `release_intent` is ≥ the bump inferred from file types
- `CHANGELOG.md` matches the generated output from all current fragments

## Bump rules

| File change | Inferred minimum bump |
|---|---|
| `src/` file modified | `patch` |
| `src/` file added | `minor` |
| `src/` file deleted or renamed | `major` |
| `tests/fixtures/distribution/*.json` | `major` |

The governance model cannot detect breaking changes inside modified files (e.g.
removing a CLI flag). In those cases declare `major` explicitly in the fragment —
declaring higher than required is allowed.
