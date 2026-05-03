---
change_id: release-pipeline-promote
release_intent: minor
summary: Add `core-ops-release promote` subcommand and wire it into CI so the post-merge release job auto-promotes [Unreleased] → [version], removes consumed fragments, and publishes the GitHub Release without manual coordination
scope: release-governance
release_preparation: false
---

The post-merge release pipeline previously required an invisible
last commit (`## [<version>] - <date>` + fragment deletion) that no
documentation called out and no validator enforced. PR #28 hit this
trap: it merged at a commit before the version section was added,
the release job's `awk` gate failed, and the v2.0.0 git tag never
landed. PR #29 retroactively fixed the CHANGELOG but couldn't undo
the workflow break.

This change closes the gap end-to-end:

- New `core-ops-release promote --version <X.Y.Z> [--date <YYYY-MM-DD>]`
  subcommand in `src/bin/core-ops-release.rs`, backed by
  `promote_changelog` in `src/core/release_governance.rs`. The function
  moves the rendered `[Unreleased]` body into a new `## [<version>] - <date>`
  section, empties the `[Unreleased]` markers back to a clean state, and
  the binary then sweeps every `.md` under `changes/` (except `README.md`).
  Idempotent on re-run.
- `.github/workflows/ci.yml` master-push job runs `promote` between
  validate and `gh release create`. The bot commits the result back to
  master with `[skip ci]` so the resulting push doesn't loop. The whole
  release job short-circuits if the tag already exists on origin, so
  re-running on a previously-shipped version is a no-op (with a one-pass
  fragment sweep for self-heal of partial prior runs).
- Documentation in `CLAUDE.md`, `AGENTS.md`, and `README.md` updated
  to describe the actual two-step contract: feature PR carries the
  `Cargo.toml` bump + fragment + rendered `[Unreleased]`; CI handles the
  promotion + fragment deletion + tag + publish on master push.

Five new unit tests cover `promote_changelog` (basic promotion, idempotence,
empty-Unreleased handling, missing-section error, missing-marker error).
