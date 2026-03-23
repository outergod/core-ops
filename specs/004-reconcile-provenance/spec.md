# Feature Specification: Provenance and Reconciliation Revision Tracking

**Feature Branch**: `004-reconcile-provenance`  
**Created**: 2026-03-23  
**Status**: Draft  
**Input**: User description: "Spec: Provenance and Reconciliation Revision Tracking 1. Summary CoreOps shall track and expose the provenance of reconciled system state so that humans and agents can explain behavioral differences across runs, builds, deployments, and UAT. This feature introduces structured tracking of: the CoreOps controller revision, the desired-state repository reference and resolved revision, the distinction between observed and successfully applied desired-state revisions, and the minimal persistent local controller state required to retain this information safely across restarts. 2. Motivation At present, behavioral differences can be difficult to attribute precisely. A host may differ because: the CoreOps controller changed, the desired-state repository changed, reconciliation observed but did not apply a newer revision, reconciliation failed after partial progress, or runtime inputs drifted independently. Without explicit provenance, UAT and debugging become interpretive guesswork. CoreOps must preserve enough operational memory to answer what changed, what was seen, what was applied, and by which controller revision. 3. Goals Make reconciled behavior attributable to both controller revision and desired-state revision. Distinguish desired-state revisions that were observed from those that were successfully applied. Persist only the minimal local state required for provenance, safety, resumability, and auditability. Expose provenance in machine-readable form. Support revision comparison during UAT and debugging. 4. Non-goals Creating a second authoritative source of desired state outside the configured source repository. Storing a full shadow copy of target truth as durable internal state. Introducing a full deployment database or orchestration control plane. Solving arbitrary drift detection beyond what reconciliation already observes. 5. Definitions Controller revision The version and source revision of the CoreOps binary or artifact performing reconciliation. Desired-state source The configured source repository or equivalent source from which CoreOps derives target state. Requested ref The symbolic ref or selector configured for the desired-state source, such as a branch, tag, or pinned revision. Observed revision The concrete desired-state revision that CoreOps most recently resolved and observed. Applied revision The concrete desired-state revision that CoreOps most recently reconciled successfully. Reconciliation generation A monotonically increasing local sequence number identifying reconcile attempts on a given controller instance or target. 6. Requirements 6.1 Provenance model CoreOps MUST track and expose the following provenance domains: Controller provenance controller version controller source revision build timestamp or equivalent build identifier build dirty/clean status, where available Desired-state provenance desired-state repository identifier requested ref last observed resolved revision timestamp of last successful observation or fetch Reconciliation provenance last attempted desired-state revision last successfully applied desired-state revision status of last reconcile attempt timestamp of last reconcile start and end reconciliation generation 6.2 Observed vs applied distinction CoreOps MUST distinguish between: a revision that has been observed or fetched, a revision that has been selected for reconciliation, and a revision that has been successfully applied. These MUST NOT be collapsed into a single 'current revision' field. 6.3 Minimal persistent local state CoreOps MAY persist minimal local controller state under a runtime data directory such as /var/lib/core-ops. Persisted local state MUST be limited to data required for: provenance, reconciliation safety, resumability, and audit/debugging. Persisted local state MUST NOT become the authoritative source of desired configuration. 6.4 Machine-readable exposure CoreOps MUST expose provenance in machine-readable form through at least one stable interface. Acceptable surfaces include: CLI status output, CLI version/build-info output, structured logs, local status files, HTTP status endpoints if available. At least one surface MUST expose all required provenance fields together. 6.5 Reconstructibility Persisted local state SHOULD be reconstructible from the desired-state source plus fresh observation, except for ephemeral transactional markers and operational history. If local state loss would render the system permanently unable to determine desired state, the design is invalid. 6.6 State schema discipline Persisted state MUST have an explicit schema version. Backward-incompatible changes to persisted state format MUST be versioned and migrated deliberately. 7. Data model A minimal conceptual model: { schema_version: 1, controller: { version: 0.9.0, revision: 8f3c2ab, build_time: 2026-03-23T10:00:00Z, tree_state: clean }, desired_state: { repository: git@example.com:org/coreops-live.git, requested_ref: main, last_observed_revision: a42be91, last_observed_at: 2026-03-23T10:05:00Z }, reconciliation: { generation: 184, last_attempted_revision: a42be91, last_applied_revision: a42be91, last_started_at: 2026-03-23T10:06:00Z, last_finished_at: 2026-03-23T10:06:09Z, status: success } } A failure case should make the divergence visible: { desired_state: { requested_ref: main, last_observed_revision: c98dd10 }, reconciliation: { last_attempted_revision: c98dd10, last_applied_revision: a42be91, status: failed } } 8. Local state layout One possible implementation shape under /var/lib/core-ops: state.json or equivalent summary state cache/ for local repo mirrors or checkouts locks/ for active reconciliation locks journal/ or equivalent event log for recent reconcile history This layout is illustrative, not normative. The important distinction is between: summary state, cache, transactional markers, and optional history. 9. Versioning policy Releasable CoreOps artifacts SHOULD follow SemVer. SemVer applies to releasable controller artifacts, not necessarily to every reconcile event or every desired-state repository revision. Desired-state revisions MUST be tracked using immutable source revision identifiers, such as commit SHAs, independently of controller release versioning. 10. Acceptance criteria This feature is complete when: CoreOps can report its own controller revision and version, CoreOps can report the desired-state repository, requested ref, last observed revision, and last applied revision, a failed reconciliation can be distinguished from a successful observation, the data survives restart, the data is machine-readable, and UAT can compare two runs and identify whether behavioral difference is attributable to controller revision, desired-state revision, or reconciliation failure/divergence. 11. Open questions These are good questions to leave explicit rather than smuggling them in: Should reconciliation history be flat-file based or SQLite-backed? Should desired-state provenance be tracked globally, per target, or both? Should CoreOps keep only summary state, or also a bounded event journal? Which machine-readable interface is canonical: CLI, file, API, or logs?"

## Clarifications

### Session 2026-03-23

- Q: Which machine-readable interface should be canonical for operator workflows when multiple interfaces exist? → A: Canonical local status file; CLI and logs may read from or mirror it.
- Q: Should reconciliation history remain summary-only in this iteration or also include a bounded event journal? → A: Summary state only; no bounded event journal yet.
- Q: Should desired-state provenance be reported only at the host level in this iteration, or also at finer target granularity? → A: Track desired-state provenance at host level only.
- Q: What exactly does observed revision mean? → A: The observed revision is the immutable revision produced by resolving the configured requested ref at the time of observation.
- Q: What is the default relationship between attempted revision and observed revision? → A: The attempted revision equals the most recently observed revision unless reconciliation logic explicitly overrides it.
- Q: How is an in-progress reconciliation represented? → A: There must be an explicit way to represent whether a reconciliation is currently running.
- Q: What does minimal state exclude? → A: Minimal state excludes derived or reconstructible data unless it is needed for performance or failure recovery.
- Q: How is the pre-reconcile state represented? → A: It must be possible to represent explicitly that no reconciliation has ever run.
- Q: Are cached repository data required for reconstructibility? → A: Cached repository data may be discarded without violating reconstructibility requirements.
- Q: Does this iteration support historical sequence analysis? → A: No. This iteration supports attribution of current state and last reconciliation outcome only.
- Q: How should the provenance domains be classified conceptually? → A: Controller provenance is identity data, desired-state provenance is observational data, and reconciliation provenance is operational state.
- Q: What persistence semantics must the canonical local status file provide? → A: It must be readable as a complete valid snapshot, updates must be atomic for readers, interrupted writes must not be treated as valid current provenance, attempted and applied revisions must remain distinct across state transitions, and schema version changes must be detectable before interpretation.
- Q: What minimum reconciliation status distinctions are required? → A: Reconciliation status must distinguish at minimum in-progress, success, and failed states. Additional values may exist, but they must not collapse those distinctions.
- Q: What ordering guarantee does reconciliation generation provide? → A: Reconciliation generation increases monotonically with each reconcile attempt.
- Q: How is divergence between attempted and observed revisions represented? → A: If the attempted revision differs from the most recently observed revision, that divergence must be explicitly represented in the provenance model.
- Q: Which persisted source is authoritative for provenance in this iteration? → A: The canonical local status file is the authoritative source of persisted provenance for this iteration, and other interfaces reflect its contents rather than maintaining independent state.
- Q: When is persisted provenance state considered valid? → A: Persisted provenance state is valid only if it is a complete snapshot with a supported schema version. Invalid or partial state is ignored and treated as absent.
- Q: How must controller versioning react to observable behavior or persisted-state compatibility changes under this spec? → A: Changes merged under this spec that alter externally observable controller behavior or persisted-state compatibility must update the controller version in Cargo.toml according to the versioning policy, and backward-incompatible persisted-state schema changes must trigger at least a minor or major version review.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Explain Current Host State (Priority: P1)

As an operator, I want CoreOps to report the controller revision, desired-state
source, last observed revision, and last applied revision for a host so that I
can explain what that host is currently running.

**Why this priority**: Without a reliable view of applied vs. observed
revisions, every later comparison or debugging workflow becomes interpretive and
slow.

**Independent Test**: Trigger a successful reconciliation, restart CoreOps, and
verify that a machine-readable status surface still reports controller
provenance, desired-state provenance, and the last successfully applied
revision.

**Acceptance Scenarios**:

1. **Given** a host that has completed a successful reconciliation, **When** an
   operator requests machine-readable provenance, **Then** the response shows
   the controller version, controller revision, desired-state source,
   requested ref, last observed revision, last applied revision, reconcile
   status, and reconcile generation together.
2. **Given** the controller restarts after a successful reconciliation, **When**
   the operator requests provenance again, **Then** the same applied revision
   and reconcile outcome remain available without needing a new successful run.

---

### User Story 2 - Distinguish Observed from Applied Revisions (Priority: P2)

As a UAT engineer, I want CoreOps to distinguish a newly observed revision from
the last successfully applied revision so that I can tell whether a behavior
change is pending, failed, or actually in effect.

**Why this priority**: This is the core behavioral distinction the current
system lacks, and it directly affects debugging failed or partial reconciles.

**Independent Test**: Cause CoreOps to observe a newer desired-state revision
and then fail reconciliation; verify that machine-readable status shows the new
observed revision, the attempted revision, the older applied revision, and a
failed reconcile status at the same time.

**Acceptance Scenarios**:

1. **Given** CoreOps observes a newer desired-state revision than the one last
   applied, **When** reconciliation fails, **Then** the reported observed
   revision and attempted revision match the newer revision while the applied
   revision remains unchanged and the status is failed.
2. **Given** CoreOps observes a newer revision but has not yet completed
   reconciliation, **When** status is requested, **Then** the system does not
   report that newer revision as applied.

---

### User Story 3 - Compare Runs Across Environments (Priority: P3)

As an operator comparing hosts or UAT runs, I want CoreOps provenance to be
stable and machine-readable so that I can compare two environments and identify
whether a behavioral difference comes from controller changes, desired-state
changes, or reconcile divergence.

**Why this priority**: Cross-run and cross-environment comparison is valuable,
but it depends on the basic provenance and applied-vs-observed model already
being in place.

**Independent Test**: Compare machine-readable provenance from two runs or
hosts and determine whether the difference is attributable to controller
revision, desired-state revision, or reconcile outcome without inspecting
internal implementation details.

**Acceptance Scenarios**:

1. **Given** two machine-readable provenance outputs from different runs or
   hosts, **When** an operator compares them, **Then** the operator can tell
   whether the difference is due to controller revision, desired-state
   revision, or reconcile status divergence.

## Non-goals

- Historical sequence analysis across multiple reconciliation events.
- A bounded or unbounded reconciliation history journal in this iteration.
- Attribution requirements beyond current state and the last reconciliation
  outcome.

---

### Edge Cases

- The desired-state source can be contacted and resolved, but reconciliation
  fails before changes are fully applied.
- A write to persisted provenance state fails or is interrupted after a new
  snapshot has begun but before it is durably complete.
- Local provenance state is missing or corrupted at startup.
- A reader encounters persisted state with an unsupported or unknown schema
  version.
- Persisted provenance exists on disk but is partial, invalid, or otherwise not
  a complete supported snapshot and must therefore be treated as absent.
- The controller binary can report a version but not full source revision
  details.
- A requested ref resolves to the same immutable desired-state revision across
  multiple runs.
- A host restarts between observation and successful apply.
- Persisted state schema changes between controller releases.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CoreOps MUST track controller provenance including controller
  version, controller source revision, build identifier or build timestamp, and
  dirty or clean build state when that information is available. Controller
  provenance is identity data.
- **FR-002**: CoreOps MUST track desired-state provenance including desired-state
  source identifier, requested ref, last observed resolved revision, and the
  time of the last successful observation. Desired-state provenance is
  observational data.
- **FR-002a**: The observed revision MUST be the immutable revision resulting
  from resolving the configured requested ref at the time of observation.
- **FR-003**: CoreOps MUST track reconciliation provenance including
  reconciliation generation, last attempted desired-state revision, last
  successfully applied desired-state revision, last reconcile status, and the
  start and end time of the most recent reconcile attempt. Reconciliation
  provenance is operational state.
- **FR-003b**: Reconciliation generation MUST increase monotonically with each
  reconcile attempt.
- **FR-003a**: The attempted revision MUST equal the most recently observed
  revision unless reconciliation logic explicitly overrides that selection.
- **FR-003c**: If the attempted revision differs from the most recently
  observed revision, that divergence MUST be explicitly represented in the
  provenance model.
- **FR-004**: CoreOps MUST distinguish observed revision, selected or attempted
  revision, and successfully applied revision as separate fields.
- **FR-005**: CoreOps MUST NOT collapse observed and applied desired-state
  revisions into a single current revision field.
- **FR-006**: CoreOps MUST persist only the minimal local state needed for
  provenance, reconciliation safety, resumability, and auditability across
  restarts.
- **FR-007**: Persisted local state MUST remain derivative and MUST NOT become
  an authoritative source of desired configuration.
- **FR-007a**: Minimal persisted state refers only to state necessary for
  provenance, reconciliation safety, and resumability, and excludes derived or
  reconstructible data unless that data is required for performance or failure
  recovery.
- **FR-008**: CoreOps MUST expose all required provenance fields together
  through at least one stable machine-readable interface.
- **FR-008a**: A local machine-readable status file is the canonical provenance
  interface for this iteration. CLI status output and structured logs MAY read
  from or mirror that status file.
- **FR-008c**: The canonical local status file is the authoritative source of
  persisted provenance for this iteration.
- **FR-008d**: Other interfaces that expose provenance MUST reflect the
  contents of the canonical local status file rather than maintaining
  independent persisted provenance state.
- **FR-008b**: The canonical local status file MUST be readable as a complete,
  valid snapshot from the perspective of readers.
- **FR-009**: Machine-readable provenance MUST allow an operator or agent to
  compare two runs and determine whether a difference is attributable to
  controller revision, desired-state revision, or reconciliation outcome.
- **FR-010**: Persisted provenance state MUST survive controller restart and
  remain readable until superseded by a later reconcile attempt.
- **FR-010a**: Updates to persisted provenance state MUST be atomic from the
  perspective of readers.
- **FR-010b**: A failed or interrupted write MUST NOT cause partial state to be
  treated as valid current provenance.
- **FR-010c**: Persisted provenance state MUST be considered valid only if it is
  a complete snapshot.
- **FR-011**: Persisted provenance state MUST include an explicit schema
  version, check that version before interpretation, treat only complete
  snapshots with supported schema versions as valid, and ignore invalid,
  partial, or unsupported persisted provenance state as absent.
- **FR-012**: Backward-incompatible changes to persisted provenance state MUST
  be versioned and migrated deliberately.
- **FR-012a**: Changes merged under this spec that alter externally
  observable controller behavior or persisted provenance compatibility MUST
  update the controller version in `Cargo.toml` according to the project's
  versioning policy.
- **FR-012b**: Backward-incompatible persisted provenance schema changes MUST
  trigger at least a minor or major controller version review according to the
  compatibility policy.
- **FR-012c**: The canonical controller version reported by this feature MUST
  come from the package version declared in `Cargo.toml`.
- **FR-013**: Loss of persisted local state MUST NOT permanently prevent CoreOps
  from determining desired state from the configured source after fresh
  observation.
- **FR-013a**: Cached repository data MAY be discarded without violating
  reconstructibility requirements.
- **FR-014**: If the latest reconcile attempt fails, CoreOps MUST preserve the
  last successfully applied revision separately from the failed attempted
  revision.
- **FR-014a**: Persisted state transitions MUST preserve the distinction
  between last attempted revision and last successfully applied revision.
- **FR-015**: Provenance reporting MUST make successful observation without
  successful apply visibly distinguishable from successful apply.
- **FR-016**: CoreOps MUST retain enough local provenance to support audit and
  debugging without storing a full shadow copy of managed host truth.
- **FR-017**: Persisted local provenance for this iteration MUST remain
  summary-only and MUST NOT require a bounded event journal.
- **FR-018**: Desired-state provenance for this iteration MUST be reported at
  the host level and MUST NOT require finer target-granularity provenance.
- **FR-019**: CoreOps MUST explicitly represent whether a reconciliation is
  currently running through the machine-readable provenance and status model.
- **FR-019a**: Reconciliation status MUST distinguish at minimum in-progress,
  success, and failed states.
- **FR-019b**: Additional reconciliation status values MAY be introduced, but
  they MUST NOT collapse or obscure the distinction between in-progress,
  success, and failed states.
- **FR-020**: CoreOps MUST explicitly represent the state in which no
  reconciliation has ever run, rather than inferring it from missing or
  malformed fields.
- **FR-021**: This iteration MUST support attribution of current state and the
  last reconciliation outcome only and MUST NOT require historical sequence
  analysis across multiple reconciliation events.

### Key Entities *(include if feature involves data)*

- **Controller Provenance**: Machine-readable identity of the CoreOps artifact
  that performed reconciliation, including version and revision metadata. This
  provenance domain is identity data.
- **Desired-State Provenance**: Machine-readable record of the configured source,
  requested ref, and most recently observed immutable desired-state revision
  obtained by resolving that requested ref at observation time. This
  provenance domain is observational data.
- **Reconciliation Record**: Machine-readable summary of the latest reconcile
  attempt, including generation, attempted revision, applied revision, status,
  timestamps, whether reconciliation is currently running, and any explicit
  divergence between the attempted revision and the most recently observed
  revision, including an explicit representation for the never-run state. This
  provenance domain is operational state. Its status model must distinguish at
  minimum in-progress, success, and failed states, and its generation value
  must increase monotonically with each reconcile attempt.
- **Persisted Provenance State**: Minimal host-local derivative state retained
  across restarts to preserve provenance and reconciliation status, excluding
  derived or reconstructible data unless needed for performance or failure
  recovery. It must be readable as a complete valid snapshot and replaced
  atomically from the perspective of readers. In this iteration, it is
  authoritatively represented by the canonical local status file.
- **Repository Cache**: Optional locally cached source data that may improve
  performance or failure recovery but is not required to preserve
  reconstructibility.
- **Schema Version**: Explicit version identifier for the persisted provenance
  format used to manage compatible and incompatible state changes.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Provenance comparison, state transition
  rules, revision distinction, and schema validation remain data-oriented logic;
  persistence, clock access, source observation, and status exposure remain in
  boundary layers.
- **Declarative state model**: Desired-state provenance, controller provenance,
  reconciliation status, and persisted summary state are defined as explicit
  data with named fields.
- **Idempotence & convergence**: Repeated runs against the same observed and
  applied revisions must preserve stable provenance, while failed attempts must
  not overwrite the last applied revision incorrectly.
- **Explicit effects/failures**: The spec requires separate reporting for
  observed, attempted, and applied revisions so that partial progress and
  failed reconciliation are not hidden.
- **Observability**: The feature centers on machine-readable provenance and
  reconciliation status that support dry-run analysis, auditing, and UAT
  comparison.
- **Provenance & traceability**: The feature directly implements the
  constitution requirement that runtime behavior be attributable to controller
  revision, desired-state revision, and reconcile outcome.
- **Safe defaults**: Persisted local state is explicitly derivative and bounded,
  reducing the risk of creating a second hidden source of truth.
- **Compatibility**: Persisted provenance data requires explicit schema
  versioning and deliberate migration for incompatible format changes.
- **Release version policy**: Any change to externally observable
  controller behavior or persisted provenance compatibility must evaluate and
  update the controller version in `Cargo.toml`, with backward-incompatible
  schema changes requiring at least a minor or major version review.
- **Test contract**: Tests must cover successful apply, failed reconcile,
  restart survival, machine-readable exposure, and comparison across runs.
- **Regenerability**: Stable provenance structures and behavioral tests allow
  the feature to be reimplemented without preserving incidental storage
  internals.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of successful reconciliation scenarios covered by feature
  acceptance tests, CoreOps reports controller provenance, desired-state
  provenance, reconcile status, and last applied revision together through a
  machine-readable interface.
- **SC-002**: In 100% of failed reconciliation scenarios covered by feature
  acceptance tests, CoreOps preserves a visible difference between the last
  attempted revision and the last successfully applied revision.
- **SC-003**: After a controller restart, provenance and reconciliation status
  remain available without data loss in 100% of restart recovery scenarios
  covered by feature tests.
- **SC-004**: During UAT, an operator can compare provenance outputs from two
  current-state or last-outcome snapshots and identify the source of
  behavioral difference within 5 minutes without inspecting internal
  implementation code.
- **SC-005**: In 100% of feature changes under this spec that alter
  externally observable controller behavior or persisted provenance
  compatibility, the merged change updates the controller version in
  `Cargo.toml` according to the project's versioning policy, and every
  backward-incompatible persisted schema change records a minor-or-major
  version review in the change artifacts.

## Assumptions

- A host-local runtime data directory is acceptable for derivative provenance
  state.
- Minimal persisted state excludes derived or reconstructible data unless it is
  justified by performance or failure recovery needs.
- Cached repository data may be discarded without violating reconstructibility
  requirements.
- Controller revision metadata is available at build or release time for normal
  releases, even if some fields may be absent in exceptional builds.
- The canonical controller version exposed by this feature is the package
  version declared in `Cargo.toml`.
- Changes merged under this spec that alter externally observable controller
  behavior or persisted provenance compatibility update the controller version
  in `Cargo.toml` according to the project's versioning policy.
- Backward-incompatible persisted provenance schema changes trigger at least a
  minor or major controller version review according to the compatibility
  policy.
- Desired-state sources provide immutable revision identifiers suitable for
  comparison.
- Observing desired state includes resolving the configured requested ref to an
  immutable revision identifier at the time of observation.
- Reconciliation normally attempts the most recently observed revision unless
  explicit reconciliation logic selects a different revision.
- If reconciliation logic selects an attempted revision different from the most
  recently observed revision, the machine-readable provenance explicitly
  represents that divergence rather than leaving it implicit.
- Reconciliation generation advances monotonically for each reconcile attempt.
- The machine-readable reconciliation state includes an explicit in-progress
  representation rather than inferring it indirectly from timestamps or logs.
- The reconciliation status model distinguishes at minimum in-progress,
  success, and failed states even if additional status values are later added.
- The machine-readable reconciliation state also includes an explicit
  never-run representation for hosts that have not yet completed or attempted a
  reconciliation.
- One stable machine-readable interface is sufficient for this iteration so
  long as it exposes all required provenance fields together.
- The canonical machine-readable interface for this iteration is a local status
  file, with CLI and logs treated as secondary views over the same provenance.
- Other provenance-reporting interfaces do not maintain independent persisted
  provenance state; they reflect the contents of the canonical local status
  file.
- Readers only treat persisted provenance as current when it is a complete
  valid snapshot with a detectable supported schema version.
- Readers ignore invalid, partial, or unsupported persisted provenance and
  treat it the same as absent persisted provenance.
- This iteration uses summary-only persisted provenance rather than a bounded
  event journal.
- This iteration supports attribution of current state and the last
  reconciliation outcome only, not historical sequence analysis across
  multiple reconciliation events.
- Desired-state provenance is host-scoped in this iteration rather than
  target-scoped.
- Controller provenance is treated as identity data, desired-state provenance
  as observational data, and reconciliation provenance as operational state.

## Open Questions
