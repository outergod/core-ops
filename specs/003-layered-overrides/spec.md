# Feature Specification: Layered Overrides for Reusable Desired State

**Feature Branch**: `[003-layered-overrides]`  
**Created**: 2026-03-20  
**Status**: Draft  
**Input**: User description: "Specify the next iteration of the Fedora CoreOS Quadlet GitOps controller around reusable desired state through native layered overrides rather than a custom templating language. Context: The current iteration already supports unattended systemd operation, journald audit integration, secure remote Git access, and reconciliation of container, volume, and socket artifacts on a single host. The next iteration should make desired state reusable across hosts while staying close to native Quadlet and systemd mechanisms. Goals: - allow shared service definitions to be reused across multiple hosts - support host-specific customization without introducing a general templating language - preserve native system semantics, deterministic evaluation, idempotence, and observability Core design direction: - reuse should be achieved through layered overrides - Quadlet-managed artifacts should support Quadlet-native drop-ins such as artifact.container.d/*.conf - native systemd artifacts should support systemd drop-ins such as artifact.socket.d/*.conf - the controller should evaluate shared base artifacts plus host-specific drop-ins into concrete desired state before diff/plan/apply - avoid custom placeholder syntax and avoid `${...}`-style substitution because it may collide with native systemd semantics Requirements: - define a repository model for shared artifacts and per-host overlays/drop-ins - define how the controller determines host identity for selecting host-specific overlays - define merge/evaluation semantics for base artifacts and drop-ins - distinguish clearly between Quadlet-native drop-ins and native systemd drop-ins - ensure the evaluation phase is deterministic, side-effect free, and testable - preserve the existing reconciliation model after evaluation produces concrete desired state - support common real-world reuse scenarios such as shared Traefik definitions with slight host-level variations Constraints: - do not introduce a general templating language - do not introduce arbitrary expression evaluation, loops, or conditionals - do not reuse `${...}` as a controller-specific parameter syntax - preserve functional core / imperative shell architecture - preserve explicit failure behavior and clear operator diagnostics - remain within the project’s existing scope of native host primitives rather than becoming a generic configuration management system Non-goals: - fleet coordination or cross-host orchestration - secret distribution systems - generic host file content management beyond the existing supported artifact model - arbitrary environment-file management as a first-class abstraction - full template engines or Helm-like rendering The specification should define: - the user/operator problem this iteration solves - the repository structure and desired-state model - evaluation/merge semantics - validation and failure rules for conflicting or invalid overlays - acceptance criteria for shared base definitions with host-specific overrides - risks and open questions for implementation"

## Clarifications

### Session 2026-03-20
- Q: How should host service selection be expressed in host.yaml? → A: Explicit service list only (no groups/roles).
- Q: What ordering should be used when applying drop-ins? → A: Use native lexicographic drop-in ordering; host overrides apply after base drop-ins.
- Q: How should evaluation handle a host selecting a service that is not defined? → A: Fail evaluation with an explicit error.
- Q: How should the controller determine host identity for selecting hosts/<host>? → A: Hostname default + explicit CLI/env override.


## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reuse shared base definitions across hosts (Priority: P1)

As an operator, I want to define shared base artifacts once and reuse them across
multiple hosts while selecting which shared services each host should include so
that consistent services can be deployed without duplication or overreach.

**Why this priority**: Reuse is the central value of this iteration and enables
consistent operations across hosts without a templating language.

**Independent Test**: Apply the same repository to two hosts with different
service selections and verify each host converges only to its selected shared
services, with no host-specific overlays present.

**Acceptance Scenarios**:

1. **Given** a repository with base artifacts and service selections, **When** a
   host evaluates desired state, **Then** the evaluation output contains only
   the base artifacts selected for that host.
2. **Given** the same base repository, **When** two hosts evaluate desired
   state with different selections, **Then** each produces a concrete desired
   state limited to its selected services.

---

### User Story 2 - Apply host-specific drop-ins without templating (Priority: P2)

As an operator, I want host-specific drop-ins to override shared base artifacts
and config payloads without using a templating language so that each host can
adjust behavior using native Quadlet/systemd mechanisms and bounded host paths.

**Why this priority**: Host-level customization is the key practical need for
reusable desired state while preserving native semantics.

**Independent Test**: Add a host-specific drop-in and a config override and
verify only that host’s concrete desired state and config materialization change,
while other hosts remain unchanged.

**Acceptance Scenarios**:

1. **Given** base artifacts plus a host-specific drop-in directory, **When** the
   host evaluates desired state after selecting services, **Then** the drop-in
   is applied only to selected base artifacts in the resulting concrete desired
   state.
2. **Given** multiple hosts with different overlays, **When** each host evaluates
   desired state, **Then** each host’s concrete desired state includes only its
   own overlays.
3. **Given** shared config payloads plus a host-specific config override,
   **When** evaluation runs, **Then** the config materialized for that host
   reflects base payloads with host overrides layered by replacement or
   directory overlay rules.

---

### User Story 3 - Deterministic, testable evaluation (Priority: P3)

As an operator, I want evaluation to be deterministic and side-effect free so
that results are testable, repeatable, and safe to run before apply.

**Why this priority**: Deterministic evaluation preserves GitOps trust and makes
reconciliation outcomes predictable.

**Independent Test**: Run evaluation twice for the same host and verify identical
outputs and stable diagnostics.

**Acceptance Scenarios**:

1. **Given** unchanged base and overlay inputs, **When** evaluation runs twice,
   **Then** the resulting desired state is byte-for-byte identical.
2. **Given** an invalid overlay, **When** evaluation runs, **Then** it fails
   explicitly with a clear diagnostic before planning or applying.

### Edge Cases

- Two overlays define conflicting keys for the same artifact.
- Host identity cannot be determined.
- Drop-in directory exists but contains no valid files.
- A drop-in targets a base artifact that does not exist.
- A host selects a service that is not defined in the base repository.
- Overlays introduce unsupported file types or extensions.
- A service is deselected while managed config files still exist on disk.
- A managed config path is renamed or removed between revisions.
- Apply fails after writing some config files but before completing all deletes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST support reusable base artifacts that can be shared
  across multiple hosts without duplication.
- **FR-002**: The system MUST support host-specific overlays using native drop-in
  formats for Quadlet artifacts (`artifact.container.d/*.conf`, `artifact.volume.d/*.conf`)
  and native systemd drop-ins for socket units (`artifact.socket.d/*.conf`).
- **FR-003**: The system MUST define a repository model that distinguishes base
  artifacts from host-specific overlays and drop-ins.
- **FR-004**: The system MUST determine host identity using the OS hostname by
  default, with an explicit CLI/env override for selecting hosts/<host>.
- **FR-005**: The repository model MUST include a host-level service selection
  mechanism where host.yaml declares an explicit service list (no groups/roles).
- **FR-006**: The system MUST evaluate base artifacts plus applicable overlays
  into a concrete desired state before diff/plan/apply, applying native drop-in
  ordering (lexicographic) with host overrides layered after base drop-ins.
- **FR-007**: Evaluation MUST be deterministic, side-effect free, and testable.
- **FR-008**: The system MUST reject overlays that target nonexistent base
  artifacts, invalid file types, or host selections that reference undefined
  services, with explicit diagnostics.
- **FR-009**: The system MUST preserve the existing reconciliation model once
  evaluation produces concrete desired state.
- **FR-010**: The system MUST NOT introduce a templating language or custom
  placeholder substitution.
- **FR-011**: The system MUST clearly distinguish Quadlet drop-ins from native
  systemd drop-ins in validation and evaluation rules.
- **FR-012**: Selected services MAY include managed config files and directories
  that are materialized to bounded host paths (e.g., `/etc/<service>`) and then
  mounted or referenced by Quadlets.
- **FR-013**: Config payloads MUST support shared base definitions plus
  host-specific overrides using whole-file replacement and directory layering.
- **FR-014**: The system MUST NOT implement semantic merging of TOML/YAML/JSON
  config formats and MUST preserve explicit boundaries to avoid becoming a
  general configuration management system.
- **FR-015**: Managed config roots for selected services (e.g., `/etc/<service>/`)
  are authoritative, closed-world directories for core-ops and MUST NOT contain
  unmanaged files.
- **FR-016**: During reconciliation, any file within a managed config root that
  is not present in the concrete desired state MUST be removed.
- **FR-017**: When a service is deselected for a host, all managed config files
  previously materialized for that service MUST be removed.
- **FR-018**: Deletions of managed config files MUST be explicit in plan/apply
  output and constrained to the managed config roots for selected services.
- **FR-019**: Partial apply or failed reconciliation MUST leave the system in a
  recoverable state; a subsequent apply MUST converge by re-applying desired
  config files and re-attempting deletions.

### Key Entities *(include if feature involves data)*

- **Repository Model**: The structure that separates shared base artifacts and
  per-host overlays.
- **Host Identity**: The identifier used to select host-specific overlays.
- **Host Declaration**: The `hosts/<host>/host.yaml` file that declares the
  host identity and selected services.
- **Base Artifact**: A shared Quadlet or systemd unit definition.
- **Drop-in Overlay**: A host-specific override applied via native drop-in
  mechanisms.
- **Service Selection**: A host-scoped declaration of which shared services are
  included in desired state.
- **Evaluated Desired State**: The concrete output after base + drop-in
  evaluation, passed to planning and apply.
- **Service Catalog**: The repository `services/` directory containing reusable
  base artifacts and their native drop-ins.
- **Host Overlay Set**: A `hosts/<host>/overrides/` tree containing host-specific
  drop-ins layered after service selection.
- **Config Payload**: Managed files/directories for a service that are
  materialized to bounded host paths and referenced by Quadlets.
- **Config Overlay**: Host-specific config files/directories that replace or
  layer on top of base config payloads.

## Repository Structure Example

```text
repo/
├── services/
│   ├── traefik/
│   │   ├── quadlet/
│   │   │   ├── traefik.container
│   │   │   └── traefik.socket
│   │   ├── quadlet-overrides/
│   │   │   └── traefik.container.d/
│   │   │       └── 10-defaults.conf
│   │   └── config/
│   │       └── etc/
│   │           └── traefik/
│   │               ├── traefik.toml
│   │               └── dynamic/
│   │                   └── routers.toml
│   │
│   ├── immich/
│   │   ├── quadlet/
│   │   │   ├── immich.container
│   │   │   └── immich.volume
│   │   └── quadlet-overrides/
│   │       └── immich.container.d/
│   │           └── 10-defaults.conf
│   │
│   ├── vector/
│   │   └── quadlet/
│   │       └── vector.container
│   │
│   └── whoami/
│       └── quadlet/
│           └── whoami.container
│
├── hosts/
│   ├── kadath/
│   │   ├── host.yaml
│   │   └── overrides/
│   │       ├── quadlet/
│   │       │   └── traefik.container.d/
│   │       │       └── 20-host.conf
│   │       └── config/
│   │           └── etc/
│   │               └── traefik/
│   │                   ├── traefik.toml
│   │                   └── dynamic/
│   │                       └── local.toml
│   │
│   ├── rlyeh/
│   │   ├── host.yaml
│   │   └── overrides/
│   │       └── quadlet/
│   │           └── vector.container.d/
│   │               └── 20-host.conf
│   │
│   └── ulthar/
│       ├── host.yaml
│       └── overrides/
│           └── quadlet/
│               └── whoami.container.d/
│                   └── 20-host.conf
│
└── README.md
```

Example `hosts/kadath/host.yaml`:

```yaml
host: kadath
services:
  - traefik
  - immich
```

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Evaluation is pure and deterministic;
  filesystem and systemd interactions remain in boundary layers.
- **Declarative state model**: Base, overlay, and evaluated desired state are
  explicit data structures.
- **Idempotence & convergence**: Repeated evaluation and apply produce stable
  outputs with no unintended changes.
- **Explicit effects/failures**: Overlay conflicts and invalid inputs fail with
  clear diagnostics.
- **Observability**: Evaluation output and overlay application are reported in
  plan/audit data.
- **Safe defaults**: No overlay is applied unless explicitly scoped to the host.
- **Compatibility**: Existing single-host reconciliation remains unchanged after
  evaluation produces concrete state.
- **Test contract**: Tests cover overlay evaluation, conflict handling, and
  deterministic output.
- **Regenerability**: Specs and tests define evaluation semantics explicitly.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of evaluation runs on unchanged inputs produce identical
  concrete desired state.
- **SC-002**: 100% of invalid overlay inputs fail during evaluation with explicit
  diagnostics before planning.
- **SC-003**: Operators can apply a shared base plus host-specific service
  selection and overlays to at least three hosts without duplicating base
  artifacts.
- **SC-004**: Evaluation adds no more than 1 second of overhead per 50 artifacts
  on a single host.

## Assumptions

- Host identity can be determined from a stable, operator-configurable source.
- Service selection can be declared in the repository per host.
- Operators will keep base artifacts and overlays in a single repository.

## Risks & Open Questions

- Determining a host identity source that is stable across reboots and
  re-provisioning may require explicit operator configuration.
- Overlay conflicts may be common in real-world repos; diagnostics must remain
  clear and actionable.
- Native drop-in semantics differ between Quadlet and systemd; evaluation rules
  must avoid surprising merges.
