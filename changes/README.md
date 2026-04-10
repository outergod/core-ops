# Release Fragments

CoreOps uses checked-in release fragments to declare machine-checkable release
intent for releasable pull requests.

Each releasable change set must add or update exactly one fragment at:

- `changes/<change-id>.md`

Current fragment format:

```md
---
change_id: example-change
release_intent: patch
summary: Short human-readable release note
scope: optional-scope
release_preparation: false
---
```

Notes:

- `release_intent` must be one of `patch`, `minor`, or `major`
- `release_preparation: true` is required only for intentional metadata-only
  release-preparation changes
- `CHANGELOG.md` is generated from approved fragment content
