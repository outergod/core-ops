# Research: Distribution Readiness

## Decision: Scope initial outside consumption to binary-only distribution

**Rationale**: The feature goal is first outside consumption with the smallest
credible support surface. Direct binary distribution satisfies the install and
verification goals without expanding scope into RPM layering, repository
metadata, or package-manager lifecycle work.

**Alternatives considered**:
- Add RPM artifacts immediately: rejected because packaging lifecycle and host
  integration semantics would materially expand the feature.
- Delay all distribution work until both RPM and binary flows are ready:
  rejected because it blocks useful outside consumption on a broader packaging
  milestone.

## Decision: Make the project entrypoint the authoritative public credibility surface

**Rationale**: Outside evaluators start at the project entrypoint. Keeping the
required signals in one stable location reduces search cost and makes drift
detectable. The entrypoint must expose latest release identity, release-gate
status, accepted verification status, and available binary artifacts in a
consistently locatable structure.

**Alternatives considered**:
- Scatter credibility signals across release pages and docs only: rejected
  because locatability would become release-dependent.
- Publish raw CI badges without a structured entrypoint section: rejected
  because badges alone do not create a stable public contract.

## Decision: Treat the authoritative verification environment as a versioned public contract

**Rationale**: Self-hosted runner drift can invalidate release confidence even
when the workflow still passes. The release story therefore needs a documented,
reproducible, and at least loosely versioned environment identity that can be
compared over time.

**Alternatives considered**:
- Keep runner details implicit inside CI configuration: rejected because drift
  would be hard to detect or explain.
- Require fully immutable infrastructure before releasing: rejected because it
  overconstrains the initial distribution milestone.

## Decision: Use a cold-start Fedora CoreOS install-and-verify path as the supported operator flow

**Rationale**: The spec defines Fedora CoreOS as officially supported. The
distribution story must therefore prove that a new operator can acquire the
binary, install it, run a first command, and perform a reproducible smoke-test
or convergence check on a freshly provisioned supported host without undeclared
setup.

**Alternatives considered**:
- Document only maintainer-oriented local development steps: rejected because
  they do not satisfy outside consumption.
- Base the public verification story on internal verification harness
  infrastructure: rejected because users cannot rely on internal project
  runners.

## Decision: Distinguish AI-assisted authorship from runtime guarantees explicitly

**Rationale**: Outside consumers need to understand that AI affects how the
project is produced, not how it behaves at runtime. Guarantees are provided by
the spec, tests, and release gate, not by trust in the authoring method.

**Alternatives considered**:
- Omit AI disclosure: rejected because the proposal explicitly calls for it.
- Mention AI without explaining guarantees: rejected because it would create
  unnecessary doubt about runtime behavior.

## Decision: Make AGPLv3+, code of conduct, and Keep a Changelog part of the public contract

**Rationale**: Distribution readiness is not only executable software. Outside
consumption also depends on clear licensing, community expectations, and
release history continuity. These should be visible and enforced as required
materials.

**Alternatives considered**:
- Defer community and release-history documents until later: rejected because
  public consumption starts now.
- Mention changelog or conduct informally in docs only: rejected because these
  are intended to become stable project obligations.
