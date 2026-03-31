# Follow-Ups

Deferred implementation work and discoveries that should be revisited after the active spec work is complete.

## CLI Revision Input

- Status: Deferred until after current spec implementation
- Area: Git revision resolution in `core-ops plan` / `core-ops apply`
- Discovery:
  - Human-readable output shortens target revisions to 8 characters for display, for example `454ac5f1`.
  - Those short revisions are not reliably accepted as `--rev` input.
  - Current loader behavior in `src/io/repo.rs` uses `git fetch origin <rev>` followed by checkout from `FETCH_HEAD`, so `--rev` currently behaves like a fetchable ref rather than a general Git-resolvable revision.
  - Full SHAs, branch names, tags, and supported revision expressions such as `main~1` work; short SHAs may fail even when unambiguous in the source repository.
- Desired follow-up:
  - Make short commit IDs accepted as `--rev` input when they are unambiguous in the source repository.
  - Keep displayed short revisions and accepted CLI revision syntax aligned enough that operator expectations are not violated.
- Likely implementation direction:
  - Resolve candidate revisions with Git after clone/fetch instead of treating all plain inputs as fetch refspecs only, or add an explicit short-SHA resolution path before checkout.
