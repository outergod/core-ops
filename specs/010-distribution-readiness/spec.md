# Feature Specification: Distribution Readiness

**Feature Branch**: `010-distribution-readiness`  
**Created**: 2026-04-07  
**Status**: Draft  
**Input**: User description: "Use docs/distribution-readiness-proposal.md as input."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Understand And Trust The Project (Priority: P1)

As a competent new operator evaluating CoreOps, I need a clear top-level
project explanation so I can understand what CoreOps is for, what it is not
for, and whether it is appropriate for my environment before I install it.

**Why this priority**: Outside consumption cannot start if new users cannot
understand the project’s scope, target audience, and trust story from the
project entrypoint.

**Independent Test**: Can be fully tested by reviewing the top-level project
entrypoint and confirming it explains the project framing, goals, non-goals,
current capabilities, current limitations, trust boundaries, and audit story
without requiring prior maintainer context.

**Acceptance Scenarios**:

1. **Given** a first-time visitor opens the project entrypoint, **When** they
   evaluate whether CoreOps fits their needs, **Then** they can see the
   project’s framing, goals, non-goals, and target audience clearly.
2. **Given** a first-time visitor considers running CoreOps from a container,
   **When** they review the entrypoint documentation, **Then** they can see
   that containerized CoreOps execution is not a supported consumption method
   and why it is outside the supported contract.
3. **Given** a first-time visitor wants to understand whether their operating
   environment is supported, **When** they review the entrypoint
   documentation, **Then** they can distinguish officially supported, untested
   but theoretically compatible, and unsupported system classes.
4. **Given** a first-time visitor wants to understand current maturity,
   **When** they review the entrypoint documentation, **Then** they can see
   examples of what is possible today, what is not yet supported, and the
   intended direction of future releases.
5. **Given** a cautious operator wants to assess trust and reversibility,
   **When** they review the entrypoint documentation, **Then** they can see
   what CoreOps modifies, how users audit changes, and how recovery or reversal
   is expected to work.

---

### User Story 2 - Install And Verify A Release Candidate (Priority: P2)

As a new operator trying CoreOps outside the development circle, I need a
supported installation and verification path so I can obtain the software, run
it, and confirm that published builds behave as claimed.

**Why this priority**: Documentation alone is insufficient for outside
consumption; a distribution-ready system must provide a practical path for
installation, execution, and release verification.

**Independent Test**: Can be fully tested by following the documented
installation path for a published build and confirming the operator can obtain
the software, verify version identity, and run the supported validation flow
without maintainer intervention.

**Acceptance Scenarios**:

1. **Given** a user wants to install CoreOps on a supported system, **When**
   they follow the documented installation path, **Then** they can obtain and
   run a supported CoreOps build through the published binary distribution
   story.
2. **Given** a user wants to verify what build they are running, **When** they
   invoke supported visibility surfaces, **Then** they can see the binary
   version and associated release identity clearly.
3. **Given** a release candidate is published, **When** release-gating
   automation evaluates it, **Then** the build, verification corpus, and spec
   conformance checks all pass before the release is treated as ready.

---

### User Story 3 - Diagnose Failures And Track Changes Safely (Priority: P3)

As an operator consuming CoreOps from outside the project, I need meaningful
failure ergonomics and release history so I can reason about what failed, what
changed between releases, and what guarantees still hold.

**Why this priority**: Distribution readiness requires a usable operational
experience after installation, not just at install time.

**Independent Test**: Can be fully tested by reviewing the published operator
surfaces and release materials and confirming they provide actionable errors,
auditable version identity, and a maintained changelog for externally visible
changes.

**Acceptance Scenarios**:

1. **Given** an operator encounters a failure, **When** CoreOps exits or emits
   diagnostics, **Then** the operator receives meaningful exit behavior,
   actionable error messaging, and logs that can be reasoned about.
2. **Given** an operator compares releases, **When** they review project change
   history, **Then** they can see a maintained changelog that explains
   externally relevant changes.
3. **Given** an operator audits a released build, **When** they inspect version
   and provenance surfaces, **Then** they can relate the running build to its
   binary version, source revision, and governing specification context.

### Edge Cases

- What happens when a release artifact is available but the documented install
  path is incomplete or no longer matches the published distribution form?
- How does release gating behave when accepted verification scenarios pass but
  spec conformance or determinism checks fail?
- What happens when version information is visible in one operator surface but
  missing or inconsistent in another?
- How does the project communicate that running CoreOps from a container is not
  supported, and why that path is outside the supported contract?
- How does the project distinguish officially supported Fedora CoreOS usage
  from theoretically compatible but untested systemd hosts and from absolutely
  unsupported non-systemd environments?
- How does the project communicate unsupported operating environments or
  incomplete distribution channels without overstating readiness?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The project entrypoint documentation MUST explain CoreOps using a
  clear framing statement, project goals, non-goals, and target audience.
- **FR-002**: The project entrypoint documentation MUST describe what CoreOps
  can do today, what it cannot yet do, and the intended direction of the
  system.
- **FR-003**: The project entrypoint documentation MUST state that running
  CoreOps from a container is not a supported consumption method and MUST
  explain that CoreOps is designed to reconcile and observe host-level systemd
  state directly.
- **FR-004**: The project entrypoint documentation MUST state that Fedora
  CoreOS is the officially tested and supported operating environment, that
  other systemd-based hosts are theoretically compatible but untested, and
  that non-systemd environments are unsupported.
- **FR-005**: The project entrypoint documentation MUST disclose the project’s
  AI-assisted authorship, clarify that AI affects authorship rather than
  runtime behavior directly, and explain that behavioral guarantees are
  enforced through the maintained specification, tests, and release gate.
- **FR-006**: The project entrypoint documentation MUST include a minimal trust
  story explaining what CoreOps modifies, how a user audits what happened, and
  how recovery or reversal is expected to work.
- **FR-007**: The project entrypoint documentation MUST include a recognizable
  project identity surface suitable for outside consumption, including a
  project logo or an explicit reserved logo placeholder pending final artwork.
- **FR-008**: The project entrypoint documentation MUST expose a compact
  credibility status surface for outside evaluators that includes, at minimum,
  the latest release identity, current release-gate result, accepted
  verification result, and currently available published distribution
  artifacts. This surface MUST be present in the project entrypoint and
  structured so those values remain consistently locatable across releases.
- **FR-009**: The project MUST be distributed under the GNU Affero General
  Public License version 3 or later.
- **FR-010**: The project MUST publish a code of conduct document that is
  visible from the project entrypoint for outside contributors and users.
- **FR-011**: CoreOps MUST expose the governing AGPLv3+ license in at least
  one discoverable CLI surface.
- **FR-012**: CoreOps MUST provide at least one supported installation path for
  direct binary consumption.
- **FR-013**: The published distribution forms MUST include a direct binary
  download suitable for manual consumption.
- **FR-013a**: The supported binary distribution set for this phase MUST
  include `x86_64`/`amd64` and `arm64`/`aarch64` builds.
- **FR-014**: A supported installation path MUST include a documented,
  deterministic sequence of steps that explains how to obtain the artifact,
  install it, run the first command, and validate a successful smoke-test
  outcome that results in a runnable CoreOps instance.
- **FR-014a**: The supported distribution story for this phase MUST include
  the canonical `core-ops.service` and `core-ops.timer` unit contract as part
  of the supported host-integration path for unattended execution.
- **FR-015**: A supported installation path SHOULD minimize external
  dependencies and SHOULD NOT require implicit system knowledge beyond what is
  explicitly documented.
- **FR-016**: The supported installation path and minimal operator-facing
  verification flow MUST succeed on a freshly provisioned supported
  environment without relying on undeclared preconditions.
- **FR-017**: The distribution story MUST document at least one supported
  installation flow that a competent stranger can follow without maintainer
  assistance.
- **FR-018**: The distribution story MUST include a minimal operator-facing
  verification flow that lets a user confirm correct behavior on their own
  system without requiring access to internal test infrastructure.
- **FR-019**: The minimal operator-facing verification flow MUST include at
  least one observable state change or convergence check whose expected outcome
  is explicitly defined and reproducible.
- **FR-020**: Release-gating automation MUST require successful build,
  verification-corpus execution on the authoritative environment, scenario
  schema/spec conformance validation, and determinism checks before a release
  candidate is treated as ready.
- **FR-021**: The authoritative verification environment used for release
  gating MUST be documented, reproducible, and versioned sufficiently for
  operators and maintainers to detect environment drift over time.
- **FR-022**: Published builds MUST be tied to visible release identity that
  includes the binary version, source revision, and governing specification
  context.
- **FR-023**: CoreOps MUST expose version identity through the CLI and through
  at least one runtime or diagnostic surface that operators use during normal
  troubleshooting.
- **FR-024**: The project MUST maintain a changelog in Keep a Changelog format
  that records externally relevant changes and is anchored to the project’s
  governing rules.
- **FR-025**: Failure surfaces MUST provide meaningful exit behavior,
  actionable error messaging, and logs that help operators reason about what
  happened.
- **FR-026**: Distribution-readiness documentation and release materials MUST
  state the limits of supported use clearly and MUST NOT imply support for
  unsupported orchestration, templating, or generic configuration-management
  use cases.

### Key Entities *(include if feature involves data)*

- **Distribution Story**: The published operator-facing explanation of how a
  user obtains, installs, verifies, and trusts CoreOps.
- **Authorship Disclosure**: The operator-facing explanation of AI-assisted
  authorship, including the distinction between how the project is produced and
  how its runtime guarantees are enforced.
- **License Declaration**: The operator-facing statement of the project’s
  governing distribution license for source and released artifacts.
- **Code Of Conduct**: The published community-behavior document for outside
  contributors and users.
- **Credibility Surface**: A compact public status view that helps an outside
  evaluator judge release currency, gate health, verification health, and
  artifact availability at a glance, with stable placement and structure across
  releases.
- **Installation Path**: A documented deterministic sequence for artifact
  acquisition, installation, first execution, and smoke-test validation that
  yields a runnable CoreOps instance.
- **Host Integration Units**: The canonical `core-ops.service` and
  `core-ops.timer` definitions and activation story used for unattended
  host-native execution.
- **Verification Flow**: A minimal operator-facing procedure for confirming
  that CoreOps behaves correctly on the user’s own system without relying on
  internal project infrastructure and with at least one reproducible expected
  outcome.
- **Distribution Artifact**: A published outside-consumption binary form that
  lets an operator obtain CoreOps for direct manual consumption, including its
  target architecture.
- **Release Identity**: The externally visible build identity tying a release
  to binary version, source revision, and governing specification context.
- **Release Gate**: The set of checks that must pass before a build is treated
  as suitable for outside consumption.
- **Authoritative Verification Environment**: The documented and versioned
  environment definition used to run release-gating verification so that
  runner drift can be detected and controlled.
- **Support Boundary**: The documented limits describing what CoreOps is for,
  what it is not for, and what environments or workflows are officially
  supported, theoretically compatible but untested, or outside the supported
  contract.

### Assumptions

- First outside consumption means early distribution readiness rather than a
  broad general-availability commitment.
- Public forge-hosted collaboration remains the delivery and automation context
  for this phase.
- The project will continue using the existing verification harness as the
  authoritative behavior-validation path for release gating.
- Distribution readiness is about packaging, documentation, release gating, and
  operator ergonomics; it does not redefine CoreOps’s fundamental convergence
  model.

## Verification Guidance *(mandatory for features that participate in the verification workflow)*

### Observable Behaviors

- A new operator can identify the project’s purpose, non-goals, support
  boundary, and installation paths from published project materials.
- A new operator can see that AI relates to how CoreOps is authored, while the
  project’s runtime guarantees are enforced through the specification, tests,
  and release gate.
- A new operator can determine that CoreOps is distributed under AGPLv3+ from
  the published project materials.
- A new operator can discover the governing AGPLv3+ license from at least one
  CLI surface without needing to inspect repository files directly.
- A new operator or contributor can locate a published code of conduct from the
  project entrypoint.
- A new operator can see that containerized CoreOps execution is unsupported
  and understand that the system is intended to operate directly against
  host-level systemd state.
- A new operator can distinguish officially supported Fedora CoreOS usage from
  theoretically compatible but untested systemd hosts and from unsupported
  non-systemd environments.
- A new operator can follow a documented installation path from artifact
  acquisition through first successful smoke-test validation without maintainer
  intervention.
- A new operator can discover and activate the canonical
  `core-ops.service`/`core-ops.timer` unattended execution path without
  needing maintainer-only knowledge.
- A new operator can complete the documented installation path without relying
  on undocumented system-specific knowledge.
- A new operator can follow a documented minimal verification flow on their own
  system and determine whether CoreOps is behaving correctly without project
  maintainer involvement.
- A new operator can execute the documented installation path and verification
  flow on a freshly provisioned supported environment without hidden setup.
- The documented operator-facing verification flow includes at least one
  explicitly expected observable state change or convergence check that can be
  reproduced by another operator on the same supported system class.
- A new operator can see compact credibility status for release currency,
  release-gate state, accepted verification state, and artifact availability
  from the project entrypoint, including the supported binary architectures.
- A new operator can locate the same credibility values in the same entrypoint
  area across releases without needing maintainer guidance.
- Release-gating automation blocks publication when required build, verification
  corpus, spec conformance, or determinism checks fail.
- The release process exposes which authoritative verification environment was
  used and whether it still matches the documented definition.
- Published distribution artifacts match the documented installation and
  verification story.
- The documented systemd service/timer integration path matches the published
  unattended execution contract.
- Published distribution artifacts include the documented `x86_64`/`amd64` and
  `arm64`/`aarch64` builds.
- Published builds expose consistent version identity across the declared
  operator-facing surfaces.
- Failure outputs remain actionable enough for an outside operator to diagnose
  common release or runtime problems.

### Invariants

- A build treated as distribution-ready MUST have passed the declared release
  gate.
- The authoritative verification environment used for release gating MUST be
  identifiable from maintained project materials.
- The credibility surface MUST refer to the same current release identity and
  gate state as the underlying published release materials.
- The credibility surface MUST refer to the same supported architecture set as
  the underlying published release materials.
- The credibility surface MUST remain consistently locatable within the project
  entrypoint across releases.
- Published project materials MUST identify the same governing license for the
  project and its released distribution artifacts.
- CLI license visibility MUST refer to the same governing AGPLv3+ license as
  the published project materials.
- Published project materials MUST provide a consistently discoverable code of
  conduct for outside-facing project use.
- Published documentation MUST NOT claim support outside the documented support
  boundary.
- Version identity exposed to operators MUST refer to the same released build
  across all declared visibility surfaces.

### Idempotency Expectations

- Re-running release-gating checks against the same release candidate under the
  same materially unchanged inputs MUST produce the same pass or fail result.
- Re-running release-gating checks in the same documented verification
  environment definition MUST not silently change the environment identity.
- Re-reading operator-facing distribution materials for the same release MUST
  not change the stated support boundary or release identity.
- Repeating the documented installation path and minimal verification flow on a
  freshly provisioned supported environment MUST not require undeclared setup
  beyond the published instructions.

### Failure Modes

- Release gating fails because the build does not complete successfully.
- Release gating fails because accepted verification scenarios or determinism
  checks do not pass.
- Release gating fails because scenario/spec conformance checks fail.
- The authoritative verification environment is undocumented, unreproducible,
  or has drifted beyond the documented versioned definition.
- Published project materials omit or misstate the governing AGPLv3+ license.
- The CLI exposes no discoverable license information or reports a license that
  differs from the published AGPLv3+ declaration.
- Published project materials omit the code of conduct or make it hard to
  discover from the project entrypoint.
- The credibility surface changes location or structure in a way that makes its
  required values hard to locate consistently across releases.
- Published artifacts do not match the documented installation story.
- A declared supported architecture is missing from the published binary
  release set.
- A documented installation path omits artifact acquisition, installation,
  first-command execution, or smoke-test validation steps.
- The distribution story omits the canonical `core-ops.service` or
  `core-ops.timer` host-integration path, or documents a unit contract that
  does not match the supported unattended execution model.
- A documented installation path depends on hidden prerequisites or implicit
  operator knowledge that are not stated in the published instructions.
- The documented installation path or operator verification flow only succeeds
  on preconditioned systems and fails on a fresh supported environment.
- The published distribution story lacks a usable operator-facing verification
  flow for confirming correct behavior on a user-managed system.
- The published operator-facing verification flow does not define a reproducible
  expected outcome for its observable state change or convergence check.
- Distribution materials suggest or imply that containerized CoreOps execution
  is a supported way to run the system.
- Distribution materials blur the line between officially supported Fedora
  CoreOS usage, theoretically compatible but untested systemd hosts, and
  unsupported non-systemd environments.
- Operator-facing documentation is incomplete, inconsistent, or does not match
  the published distribution form.
- Version identity is missing or inconsistent across declared operator-facing
  surfaces.

### Upgrade Considerations

- New releases MUST preserve a comprehensible changelog trail for outside
  operators.
- Changes to installation paths, release identity fields, or release-gating
  semantics MUST be communicated clearly across distribution materials.
- Distribution improvements MUST preserve the existing behavioral verification
  contract rather than weakening it for publication convenience.

### Verification Coverage Boundary

- The authoritative VM-backed accepted verification corpus remains required for
  CoreOps behavioral and environment-level claims that the verification harness
  can execute directly.
- Distribution-readiness obligations in this feature are primarily validated
  through integration tests, release-workflow contract tests, documentation
  contract tests, and the protected authoritative E2E gate rather than by
  adding accepted verification scenarios for documentation-only or
  release-orchestration concerns.
- New accepted scenario classes are only required for this feature if a change
  introduces new CoreOps behavioral semantics or guest-environment contracts
  that the VM-backed verification harness can prove directly.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Release criteria, support boundaries,
  and versioning rules remain explicit policy and data contracts; publishing,
  binary distribution, and CI execution remain side-effecting boundaries.
- **Declarative state model**: Distribution readiness adds declarative release
  expectations, support boundaries, and externally visible identity rules
  without changing CoreOps’s desired-state reconciliation model.
- **Idempotence & convergence**: Re-running the same release-gating flow
  against the same candidate should produce the same result under materially
  unchanged conditions.
- **Explicit effects/failures**: Release-gating failures, unsupported use
  cases, and operator-facing limitations are documented explicitly rather than
  implied.
- **Observability**: The feature requires visible version identity, actionable
  diagnostics, maintained release history, and auditable release-gating
  outcomes.
- **Provenance & traceability**: Published builds must remain attributable to
  binary version, source revision, and governing specification context.
- **Safe defaults**: Distribution materials must not overclaim support, and
  release gates must fail closed when required checks do not pass.
- **Compatibility**: Externally visible binary distribution, changelog,
  version identity, and release-gating semantics are compatibility-sensitive
  and require conservative evolution.
- **Release version policy**: This feature directly affects externally visible
  documentation, packaging, CI/CD gating, and version communication, so release
  policy review is mandatory.
- **Test contract**: Coverage must include release-gate success and failure,
  version-identity visibility, installation-path validation, and the required
  `cargo test` and `cargo clippy --all-targets -- -D warnings` gates for Rust
  changes unless explicitly exempted.
- **Regenerability**: The feature defines stable documentation, release-gating,
  and visibility expectations so future packaging and release workflows can be
  regenerated safely from maintained specs and tests.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A competent first-time evaluator can determine CoreOps’s goals,
  non-goals, support boundary, intended audience, and trust story from the
  project entrypoint within 10 minutes and without maintainer assistance.
- **SC-002**: 100% of release candidates treated as distribution-ready have
  passed the declared release gate, including build, accepted verification
  corpus, spec conformance, and determinism checks.
- **SC-003**: Operators can identify the running build’s release identity from
  every declared version-visibility surface without ambiguity in 100% of
  reviewed release candidates.
- **SC-004**: Externally relevant release changes remain traceable through a
  maintained changelog and published release materials for 100% of distributed
  builds.
