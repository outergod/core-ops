# Specification Quality Checklist: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-05-05
**Last Revised**: 2026-05-05 (scope expansion: stateless `--source-repo` flag + spec/016 example removal)
**Feature**: [spec.md](../spec.md)

## Content Quality

- [X] No implementation details (languages, frameworks, APIs)
- [X] Focused on user value and business needs
- [X] Written for non-technical stakeholders
- [X] All mandatory sections completed

## Requirement Completeness

- [X] No [NEEDS CLARIFICATION] markers remain
- [X] Requirements are testable and unambiguous
- [X] Success criteria are measurable
- [X] Success criteria are technology-agnostic (no implementation details)
- [X] All acceptance scenarios are defined
- [X] Edge cases are identified
- [X] Scope is clearly bounded
- [X] Dependencies and assumptions identified

## Feature Readiness

- [X] All functional requirements have clear acceptance criteria
- [X] User scenarios cover primary flows
- [X] Feature meets measurable outcomes defined in Success Criteria
- [X] No implementation details leak into specification

## Notes

- All checklist items pass on the second iteration. The first iteration was
  authored against the operator-approved plan in
  `/home/outergod/.claude/plans/this-is-spec-17-greedy-pancake.md`. The rewrite
  was triggered by an operator correction surfacing during user-story authoring:
  the original US1 acceptance scenario referenced `core-ops plan --repo X --rev Y`,
  flags removed by spec/015 long ago. Investigation revealed (a) my mis-inference
  came from stale documentation in `docs/follow-ups.md`, `docs/development.md`,
  and `infra/repo/README.md`, and (b) the four spec/016 in-tree example READMEs
  reference a `--source-repo` flag that has never been implemented — they shipped
  in v2.0.0 with non-runnable "Try it" instructions.
- Operator decisions on the consequential trade: scope `--source-repo` into
  spec/017 for both `plan` and `apply` (FR-010 through FR-016); remove the four
  spec/016 example fixtures as superseded by spec/017's real-world examples
  (FR-017 through FR-019); clean up the stale CLI documentation (FR-020).
- The `Verification Guidance` section from the spec template was deliberately
  omitted: this feature does not introduce new mutation classes — stateless
  apply is an entry-point variation over the existing apply path. Existing
  apply VM-backed scenarios remain authoritative. The exemption is documented
  explicitly in `Constitution Alignment → Test contract` per Principle 10.
- Stateless-mode provenance representation (e.g., the `(stateless)` sentinel
  for `desired_state.requested_ref`) is intentionally left as an assumption to
  be locked during `/speckit.plan` after a review of
  `src/core/types.rs::DesiredStateProvenance`. This is not a `[NEEDS
  CLARIFICATION]` marker because the user value (stateless invocations are
  observable in `core-ops status` and distinguished from init'd state) is fully
  specified; only the byte-level encoding is deferred.
