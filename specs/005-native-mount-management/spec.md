# Feature Specification: Native Mount Management

**Feature Branch**: `005-native-mount-management`  
**Created**: 2026-03-24  
**Status**: Draft  
**Input**: User description: "Specify the next iteration of the Fedora CoreOS GitOps controller to support native mount management for host services. Context: The current iteration already supports unattended systemd operation, journald audit integration, secure remote Git access, reconciliation of container, volume, and socket artifacts, reusable service selection, host-specific overrides, and service-owned config payloads. A current blocking use case is running Immich with media mounted from a Synology NAS over NFS. Goals: - support native systemd mount units as first-class managed artifacts - enable service definitions to depend on mounted filesystems in a native and explicit way - preserve idempotence, explicit failure behavior, observability, and native system semantics Requirements: - define support for mount units, and automount units only if clearly justified - support common host storage cases such as NFS mounts needed by selected services - define ordering and dependency semantics between mounts and services that consume mounted paths - define validation and verification rules for mount lifecycle - define bounded host directory/path preparation if required for mount targets - preserve the current reconciliation model and avoid introducing a generic storage orchestration system Constraints: - remain systemd-native - do not introduce Kubernetes-style storage abstractions - do not expand into generic network share management beyond native unit semantics - preserve functional core / imperative shell design and explicit operator diagnostics Non-goals: - three-way reconciliation in this iteration - fleet coordination - secret distribution - generic configuration management"

## Clarifications

### Session 2026-03-24

- Q: Where should mount-specific CoreOps metadata live relative to native artifacts? -> A: User-authored native `.mount` and optional `.automount` artifacts remain primary, and CoreOps-specific semantics are encoded in a small `[X-CoreOps]` section inside those artifacts rather than in a separate YAML-first mount resource model.
- Q: What is the required removal behavior for previously managed mounts? -> A: Stop dependent managed services first, then remove generated units and deactivate the mount; fail explicitly if the mount remains busy or cannot be cleanly removed.
- Q: How are service-to-mount dependencies expressed in generated native units? -> A: Use both path-based dependencies for consumed paths and explicit unit dependencies where the generated mount or automount units must be referenced directly.
- Q: How must services consuming mounted host paths express dependency semantics? -> A: The controller must materialize dependency semantics in the generated unit configuration itself using native systemd dependency mechanisms such as RequiresMountsFor, After, and Requires.
- Q: If a required mount becomes unavailable after a dependent service is already running, what should reconciliation do? -> A: Leave the running service alone, mark it degraded or blocked, and prevent future starts or restarts until the mount recovers.
- Q: Should this iteration support automount units, and if so for what scope? -> A: Support automount only for network-backed mounts such as NFS.
- Q: What bounded path preparation is allowed for mount targets? -> A: CoreOps may prepare bounded, service-consumed directories, including optional owner, group, and mode, but it must not become a general directory management system.
- Q: Should services depend on mounts by path only or by explicit mount declaration identity? -> A: Services depend on explicit mount declaration identities, and each mount declaration also includes its target path.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reconcile a mount-backed service (Priority: P1)

As an operator, I can author native mount artifacts and annotate them with a small `[X-CoreOps]` metadata section so that a service such as Immich can consume NAS-backed media without out-of-band sidecar metadata or post-reconcile shell steps, and the generated service unit carries those dependency semantics natively.

**Why this priority**: This is the blocked production use case. Without first-class mount support, services that rely on host-mounted storage cannot be managed end to end.

**Independent Test**: Can be fully tested by authoring an NFS-backed native `.mount` artifact with embedded `[X-CoreOps]` metadata plus a service that consumes the mounted path, running plan and apply, and confirming that the mount is established before the dependent service is started.

**Acceptance Scenarios**:

1. **Given** a host selects a service that includes a user-authored native mount artifact annotated with `[X-CoreOps]`, **When** the operator runs plan, **Then** the plan shows the managed mount identity, its target-path preparation if needed, and the native systemd dependency semantics that will be materialized in the dependent unit configuration, using path-based dependencies for consumed paths and explicit unit dependencies where the managed mount or automount units must be referenced directly.
2. **Given** a host has not yet mounted the declared storage, **When** the operator runs apply with valid desired state, **Then** the mount is reconciled successfully and the dependent service becomes runnable only after the mount is active.
3. **Given** the desired mount and dependent service are already active and unchanged, **When** the operator reruns plan or apply, **Then** no unintended mount or service changes are proposed.
4. **Given** a previously managed mount is removed from desired state, **When** reconciliation runs, **Then** dependent managed services are stopped first, generated mount or automount units are removed only after the mount is no longer active, and reconciliation fails explicitly if the mount remains busy or cannot be cleanly removed.

---

### User Story 2 - Diagnose mount failures explicitly (Priority: P2)

As an operator, I can see why a managed mount failed and how that affected dependent services so that I can correct the host, network, or desired-state problem without guessing.

**Why this priority**: Native mount support is only safe if failure behavior is explicit. Hidden retries or silent fallback would create operational risk.

**Independent Test**: Can be fully tested by declaring an unreachable or invalid mount, running plan and apply, and verifying that the mount failure and dependent service decision are surfaced clearly without partial success being misreported.

**Acceptance Scenarios**:

1. **Given** a declared mount source is unreachable or invalid, **When** the operator runs apply, **Then** the controller reports the mount failure explicitly and does not treat the dependent service as successfully reconciled.
2. **Given** a dependent service requires a mount that is not active or not verified, **When** reconciliation runs, **Then** the service is held back or marked failed in a way that makes the dependency failure obvious to the operator.
3. **Given** a previously failed mount becomes reachable again, **When** the operator reruns reconciliation, **Then** the controller converges without manual cleanup and reports the successful recovery.

---

### User Story 3 - Reuse mount-aware service definitions safely (Priority: P3)

As an operator, I can reuse service definitions that declare mount needs and only opt into automount behavior when it is explicitly justified, so that the controller stays systemd-native without turning into a generic storage orchestrator.

**Why this priority**: Reusable service definitions and host overrides are already part of the model. Mount support must fit those boundaries instead of adding a parallel storage system.

**Independent Test**: Can be fully tested by selecting two services with different mount needs, verifying that shared rules remain reusable, and confirming that optional automount behavior is only emitted when explicitly requested and valid.

**Acceptance Scenarios**:

1. **Given** two hosts select the same service definition but use different host-specific mount details, **When** plan is run for each host, **Then** each host receives the correct mount-backed service plan without duplicating the whole service definition, and any host override remains legible as an override of the native source artifact plus its bounded `[X-CoreOps]` metadata.
2. **Given** a service definition does not explicitly justify on-demand mounting, **When** it declares a mount dependency, **Then** the controller manages a normal mount unit and does not emit an automount unit.
3. **Given** a service definition explicitly opts into automount behavior for a network-backed mount such as NFS, **When** the operator runs plan, **Then** the resulting plan shows both the mount and its linked automount relationship, with the dependent service carrying path-based dependency semantics for the consumed path and explicit unit dependencies on the generated automount or mount units where needed.

### Edge Cases

- A declared mount target directory does not exist yet and must be prepared before mount activation, potentially with declared owner, group, and mode if those attributes are part of the bounded service-consumed directory contract.
- A mount target exists but is a regular file, symlink, or otherwise invalid for the declared mount.
- A mount source or option change requires a mount or automount unit update while a dependent service is already running, and the generated native dependencies must stay coherent across that transition.
- The managed mount artifacts are native systemd `.mount` and optional `.automount` units, not Quadlet generator inputs, and the embedded `[X-CoreOps]` section must remain valid as a systemd extension section that is ignored by systemd itself.
- A mount becomes unavailable after the dependent service was previously healthy; the running service remains in place, but reconciliation marks it degraded or blocked and prevents later starts or restarts until the mount recovers.
- A previously managed mount is removed from desired state while the mount is still active or still consumed by a running managed service.
- Two selected services reference the same mount target with conflicting mount definitions.
- A host override attempts to change a mount outside the selected services' declared scope.
- An automount declaration is present without a corresponding valid network-backed mount declaration.
- A service depends on a path below a managed mount target but the mount is active at a different location than declared.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support user-authored native mount and optional automount source artifacts as first-class managed artifacts in desired state, planning, reconciliation, status, and audit outputs.
- **FR-002**: The system MUST treat those native mount artifacts as the primary operator-authored source of mount behavior rather than requiring a separate YAML-first mount resource model.
- **FR-003**: The system MUST allow a selected service definition to declare one or more required managed mount identities, with each identity carried by a small `[X-CoreOps]` metadata section embedded in the corresponding user-authored native mount source artifact, and those required mounts MUST be available before the service is treated as successfully runnable.
- **FR-003a**: Service definitions are the authoritative source of service-to-mount relationships. Embedded `[X-CoreOps]` metadata on managed mount artifacts MUST NOT define, infer, or override consumer relationships.
- **FR-004**: The system MUST support common host storage use cases that rely on native mount semantics, including network-backed mounts such as NFS needed by selected services.
- **FR-005**: The system MUST restrict the embedded `[X-CoreOps]` metadata to CoreOps-specific reconciliation semantics that native systemd sections do not already express cleanly, including stable mount identity, bounded path-preparation metadata, managed removal policy, and any identity-level verification metadata required by reconciliation.
- **FR-006**: The system MUST derive explicit ordering and dependency behavior between managed mounts and the services that consume them so that dependent services are not started, restarted, or reported healthy before required mounts are active and verified. For services consuming mounted host paths, the controller MUST materialize those dependency semantics directly in the generated unit configuration itself using native systemd dependency mechanisms. Path-based dependency materialization via mechanisms such as `RequiresMountsFor` MUST be used for consumed mounted paths, and explicit unit dependencies such as `After` and `Requires` MUST also be emitted where the generated mount or automount units themselves need to be referenced directly.
- **FR-007**: The system MUST validate managed mount artifacts before execution, including stable embedded identity, mount target path validity, duplicate or conflicting definitions, required dependency references, consistency between the native unit contents and the embedded `[X-CoreOps]` metadata, and native-unit compatibility rules.
- **FR-008**: The system MUST verify mount lifecycle outcomes during reconciliation and distinguish at least these states: not yet applied, active and verified, degraded or unavailable, and failed validation or activation; if a required mount becomes unavailable after a dependent service is already running, the service MUST remain running but be reported as degraded or blocked, and future starts or restarts MUST be prevented until the mount recovers.
- **FR-009**: The system MUST support bounded host path preparation for declared mount targets when preparation is required to make the mount valid; that preparation MAY create the declared target path and necessary parent directories and MAY enforce declared owner, group, and mode for service-consumed directories, but it MUST remain limited to those bounded paths and MUST NOT expand into general directory management.
- **FR-010**: The system MUST preserve idempotence for mount reconciliation: reapplying unchanged desired state MUST NOT remount, restart dependent services, or rewrite units unnecessarily.
- **FR-011**: The system MUST surface mount-specific diffs, planned actions, reconciliation outcomes, and failure diagnostics through the same operator-visible plan, status, and audit channels already used for other managed artifacts.
- **FR-012**: The system MUST keep mount management within the existing declarative reconciliation model and MUST NOT introduce a generic storage orchestration layer, share discovery workflow, or non-native storage abstraction.
- **FR-013**: The system MUST allow host-specific overrides for mount-backed services only within the selected services' declared ownership boundaries, and those overrides MUST remain legible as overrides to native source artifacts and their bounded embedded metadata.
- **FR-013a**: Embedded `[X-CoreOps]` metadata MUST follow the same layering and override order as native unit content, with later effective values overriding earlier ones after layering. CoreOps MUST evaluate the effective merged `[X-CoreOps]` section after layering and MUST then validate the merged result against mount identity, ownership, preparation, and lifecycle invariants; that validation MAY be stricter than systemd parsing where reconciliation correctness requires it.
- **FR-013b**: Observed-state comparisons MUST treat retained `[X-CoreOps]` metadata as controller-managed source metadata and MUST NOT report drift when differences are limited to `[X-CoreOps]` content that systemd itself ignores, unless that metadata changes the effective merged CoreOps reconciliation semantics.
- **FR-014**: The system MUST support removing a previously managed mount identity and reconcile the host back to the declared state without leaving ambiguous ownership of the mount or automount units. When removal is required, the system MUST stop dependent managed services first, remove generated mount or automount units only after the mount is no longer active, and fail explicitly if the mount remains busy or cannot be cleanly deactivated or removed.
- **FR-015**: Automount units MAY be managed only when the desired state explicitly requests them for a declared network-backed mount such as NFS; ordinary mount units MUST remain the default, non-network mount cases MUST use normal mount units only, and automount support MUST remain bounded to native unit semantics. When automount is enabled, dependent services MUST still carry path-based dependency semantics for the consumed mounted paths, and explicit unit dependencies MUST target the generated automount or underlying mount units where required to preserve correct native ordering and activation behavior.
- **FR-016**: The system MUST reject or clearly fail reconciliation when a dependent service references a managed mount identity that is not declared by any selected service-owned native mount artifact.
- **FR-017**: The system MUST preserve explicit operator diagnostics when mount reconciliation fails, including which managed mount identity failed, why verification failed, and which dependent services were blocked or degraded as a result.
- **FR-018**: The implementation MUST treat managed mount artifacts as native systemd units rather than Quadlet generator inputs, and it MUST use `[X-CoreOps]` as the embedded metadata section name so the section is ignored by systemd according to the systemd extension-section mechanism.
- **FR-019**: Changes merged under this feature that alter externally observable mount behavior, CLI output, reconciliation semantics, or persisted compatibility MUST evaluate and update the release version policy accordingly, using the package version in `Cargo.toml` as the canonical controller version.

### Key Entities *(include if feature involves data)*

- **Managed Mount Artifact**: A user-authored native `.mount` artifact, plus an optional `.automount` companion, carrying managed mount identity and bounded `[X-CoreOps]` reconciliation metadata.
- **CoreOps Mount Metadata**: The bounded `[X-CoreOps]` metadata embedded in a managed mount artifact for identity and reconciliation-only semantics not expressed by native systemd sections.
- **Mount Dependency**: A service-declared requirement on one or more managed mount identities, materialized through native path-based and explicit unit dependencies.
- **Mount Reconciliation Result**: The operator-visible outcome for a managed mount during planning or apply, including validation status, activation result, verification result, removal result when desired state drops the managed mount, and any dependent-service impact.
- **Prepared Target Path**: A bounded host path that may need to exist before a managed mount can be activated, including the declared mount target and any necessary parent directories, plus optional owner, group, and mode metadata when that directory is explicitly service-consumed.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Mount evaluation, dependency ordering, validation, conflict detection, service-impact decisions, and bounded directory-preparation rules remain pure data transformations; filesystem preparation, unit writes, systemd reloads, and mount activation stay in explicit side-effect boundaries.
- **Declarative state model**: Desired state includes managed native mount artifacts plus embedded `[X-CoreOps]` metadata and service-to-mount dependencies by managed identity; observed state captures native mount/unit presence and verification status; plans describe mount, preparation, and service actions; outcomes report success, failure, and blocked dependencies.
- **Idempotence & convergence**: Unchanged desired mount state converges without repeated remounts or unnecessary service churn; retries after transient mount failures converge once the underlying host or network issue is fixed.
- **Explicit effects/failures**: Invalid mount definitions, failed target preparation, activation failures, blocked dependent services, and busy-or-unclean mount removals are surfaced as explicit outcomes rather than hidden retries or silent fallback.
- **Observability**: Plans, status output, and audit events include mount-specific diffs, actions, verification outcomes, and dependency-driven service decisions, including both path-based and explicit unit dependency semantics generated for mount-consuming services.
- **Provenance & traceability**: Reconciliation reports continue to tie mount behavior to controller version, desired-state revision, and reconciliation outcome so operators can explain why a mount-backed service changed behavior.
- **Safe defaults**: Normal mount units are the default; automount behavior and any action that bypasses ordinary dependency safety require explicit intent.
- **Compatibility**: The feature extends the managed artifact model conservatively, stays within native unit semantics, and documents any externally visible behavior or state compatibility changes.
- **Release version policy**: Any change merged under this feature that affects mount behavior, CLI/status output, reconciliation semantics, or persisted compatibility requires release-version-policy review and uses `Cargo.toml` as the canonical controller version.
- **Test contract**: Tests cover mount validation, dependency ordering, idempotent reapply, blocked-service behavior, recovery from transient mount failure, override boundary enforcement, and operator-visible diagnostics.
- **Regenerability**: Stable managed mount artifact and embedded `[X-CoreOps]` metadata contracts keep the feature replaceable by spec-driven regeneration without preserving incidental implementation structure.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Operators can author an NFS-backed managed mount artifact and reach a correct first plan for the host without sidecar metadata files, manual unit editing after selection, or ad hoc shell steps in 100% of acceptance scenarios.
- **SC-002**: In 100% of failure test scenarios, a dependent service is not reported healthy or started before its required mount is active and verified.
- **SC-003**: In regression scenarios where desired mount and service state are unchanged, repeated reconciliation produces zero unexpected mount or dependent-service actions.
- **SC-004**: In 100% of mount failure and recovery scenarios, operators can determine from one reconciliation attempt and its diagnostics which mount failed, whether it later recovered, and which services were affected.

## Assumptions

- Native mount support is scoped to host mounts expressed through native unit semantics rather than a new storage abstraction layer.
- Ordinary mount units are the default for this iteration; automount support is limited to explicitly declared network-backed mount cases such as NFS.
- Bounded path preparation may include optional owner, group, and mode only for declared service-consumed directories associated with managed mounts.
- Secret material required to access remote storage remains outside the scope of this feature and continues to be handled by existing operator workflows.
- The feature extends the existing reusable service-selection and host-override model rather than replacing it.
- Managed mount identities are embedded in native source artifacts through a bounded `[X-CoreOps]` section; services depend on those identities rather than on raw paths alone.
- Embedded `[X-CoreOps]` metadata follows native unit layering semantics, with later entries overriding earlier ones before CoreOps validates the effective merged result.
- Deployed native unit artifacts may retain the `[X-CoreOps]` section unchanged, and CoreOps runtime handling must parse and validate the effective merged section rather than requiring a separate stripped deployment form.
