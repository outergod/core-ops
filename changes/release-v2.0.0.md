---
change_id: release-v2.0.0
release_intent: patch
summary: Promote [Unreleased] to [2.0.0] - 2026-05-03 and delete the spec/016 fragment
scope: release-prep
release_preparation: true
---

Pure release-prep PR. The original spec/016 PR (#28) was merged before
its [Unreleased] → [2.0.0] CHANGELOG promotion landed, so master ended
up with the bullet still in [Unreleased] and no `## [2.0.0]` section
to anchor the v2.0.0 tag. This PR closes that gap:

- Move the spec/016 Changed bullet from the machine-managed
  [Unreleased] block into a new `## [2.0.0] - 2026-05-03` section.
- Empty the [Unreleased] block back to just its markers, matching
  the rendered output once the consumed fragment is dropped.
- Delete `changes/016-source-repository-layout.md` per the fragment
  lifecycle ("Release tagged and published — Delete the fragment").
  The CHANGELOG entry is now the durable record of the change.

`release_preparation: true` declares this as the metadata-only flow,
so `core-ops-release validate --base-ref master` accepts the no-op
version bump.

After this merges, tag v2.0.0 against the merge commit; CI publishes
the GitHub Release using the new [2.0.0] section as release notes.
