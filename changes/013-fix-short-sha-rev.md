---
change_id: 013-fix-short-sha-rev
release_intent: patch
summary: Fix short commit ID acceptance as --rev CLI input
scope: cli-revision
release_preparation: false
---

Short commit SHAs displayed in human-readable apply/plan output (e.g. `454ac5f1`) are
now accepted as `--rev` input. Previously, the loader treated all revision inputs as
fetchable refspecs (`git fetch origin <rev>`), which fails for short and full SHAs on
most Git servers. The fix detects hex-only revision inputs and resolves them against
objects already present from the initial clone instead.
