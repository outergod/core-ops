# Feature Specification: Source Repository Layout Formalization

**Feature Branch**: `016-source-repository-layout`
**Created**: 2026-05-01
**Status**: Draft
**Input**: User description: "Formalize the CoreOps source repository layout: payload-kind directories (quadlet/, systemd/, config/, extensible), eliminate redundant etc/<svc>/ mirroring via optional service.yaml config-root, eliminate overrides/ in hosts (host overlay mirrors service shape directly under hosts/<id>/<svc-id>/), hard cut of legacy quadlets/ parser, plus core-ops skill install subcommand and reference example repos."

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Author a service from spec alone (Priority: P1)

An operator with no prior knowledge of CoreOps internals adds a new container-backed service to their source repository. They consult only this spec and one reference example. They place a Quadlet container file, a native socket, a config file, and a host overlay drop-in in their canonical locations. The loader accepts the result and produces a plan that targets the correct host paths.

**Why this priority**: This is the entire reason the layout is being formalized. If an operator cannot author a conformant service from the spec alone, the formalization has failed regardless of any other property.

**Independent Test**: A reviewer is given the spec and one example. They author a fresh service and host overlay in a scratch repo. `core-ops plan` succeeds and the planned actions match expectations on first attempt without code-reading.

**Acceptance Scenarios**:

1. **Given** an empty source repository, **When** the operator adds `services/whoami/quadlet/whoami.container`, `services/whoami/config/whoami.toml`, and a host that selects `whoami`, **Then** plan output shows `whoami.container` deployed to the host Quadlet directory and `whoami.toml` deployed to `/etc/whoami/whoami.toml`.
2. **Given** a service whose deployment target differs from its identifier, **When** the operator adds `services/traefik-dnschallenge/service.yaml` containing `config-root: traefik` and a config file under `services/traefik-dnschallenge/config/`, **Then** the file is planned for `/etc/traefik/<filename>`, not `/etc/traefik-dnschallenge/<filename>`.
3. **Given** a service with multiple units, **When** the operator places a drop-in at `services/<svc>/quadlet/<unit>.container.d/90-base.conf`, **Then** plan output reflects the drop-in scoped to that exact unit.

---

### User Story 2 — Install the agent skill into a source repository (Priority: P2)

An operator runs `core-ops skill install` in their source repository. A skill bundle is written to `.agents/skills/core-ops-source-repo/`. An agent (LLM, IDE assistant, or human reading `SKILL.md`) opens the source repo and authors a new service or host overlay. The loader accepts the agent's output without further coaching.

**Why this priority**: The skill operationalizes the formal layout for agents and humans. It is independently valuable from the layout spec itself, but it derives from the spec — so it ships in the same feature.

**Independent Test**: After `core-ops skill install`, an external agent (with no other CoreOps context) authors a service from the bundled `SKILL.md`. The loader accepts the result on first attempt. Removing the bundle (deleting `.agents/skills/core-ops-source-repo/`) returns the source repo to its prior state with no residue elsewhere.

**Acceptance Scenarios**:

1. **Given** a source repository, **When** the operator runs `core-ops skill install`, **Then** `<cwd>/.agents/skills/core-ops-source-repo/SKILL.md` exists and contains the canonical layout, the `service.yaml` schema, the payload-kind dispatch table, and a worked authoring walk-through.
2. **Given** the same operator, **When** they run `core-ops skill install --global`, **Then** `~/.agents/skills/core-ops-source-repo/` is populated equivalently.
3. **Given** the same operator, **When** they run `core-ops skill install --print`, **Then** the bundle contents are written to standard output (no filesystem writes) in a form a shell pipe can redirect or extract.
4. **Given** an agent that has only the installed skill bundle, **When** asked to author a service that satisfies a stated need (e.g. "add a redis container with a config file"), **Then** the loader accepts the agent's output without modification.

---

### User Story 3 — Migrate the existing live source repository (Priority: P3)

The operator has a live source repository running the legacy layout. They migrate it to the formalized layout in one mechanical pass. After migration, `core-ops plan` against the migrated repository produces the same set of host destination paths as before. No service silently relocates its files.

**Why this priority**: The migration is one-time, bounded to a single known repository, and recoverable from version control. It must work, but it does not gate authoring or skill install.

**Independent Test**: Take the legacy `~/code/ulthar/repo/`. Apply the documented migration (script or sequence). Run `core-ops plan` on the new repo. Compare the planned destination paths against a snapshot of the same plan from the legacy layout. The destination set is identical; only source paths differ.

**Acceptance Scenarios**:

1. **Given** the legacy repository at `~/code/ulthar/repo/`, **When** the migration is applied, **Then** every file previously under `services/<svc>/quadlet/` is now at `services/<svc>/quadlet/` (unchanged), every file previously under `services/<svc>/quadlet-overrides/<unit>.<ext>.d/` is now at `services/<svc>/quadlet/<unit>.<ext>.d/`, every file previously under `services/<svc>/config/etc/<svc>/` is now at `services/<svc>/config/`, and every file previously under `hosts/<h>/overrides/<kind>/...` is now at `hosts/<h>/<svc>/<kind>/...`.
2. **Given** the migrated repository, **When** the operator runs `core-ops plan`, **Then** the planned destination paths match the pre-migration plan exactly.
3. **Given** any service whose pre-migration destination root differed from its identifier (e.g. `traefik-dnschallenge` → `/etc/traefik/`), **When** migration is applied, **Then** a `service.yaml` is generated declaring the correct `config-root` so the destination remains stable.

---

### Edge Cases

- A service identifier or payload-kind directory name begins with `_` or `.` (reserved namespace).
- A `service.yaml` contains keys outside the schema (forward-incompatible additions are intentional).
- A host overlay attempts to introduce a base unit file (e.g. a `*.container` file directly under `hosts/<h>/<svc>/quadlet/`).
- A `config/` file's normalized destination escapes `/etc/<config-root>/` (e.g. via `..` or absolute symlink targets present in the source).
- Two distinct files across services compute to the same host destination path.
- A host's `host.yaml` selects a service whose directory does not exist in `services/`.
- A `service.yaml` is malformed (invalid YAML, wrong type for `config-root`, etc.).
- A source repository contains a top-level `quadlets/` directory or a `services/<svc>/quadlet-overrides/` directory (legacy artifacts).
- A drop-in file at `<unit>.<ext>.d/<file>.conf` exists but its parent unit file does not exist anywhere in the merged set (orphan drop-in).
- Two services declare the same `config-root` but contribute non-overlapping filenames (allowed) versus the same filename (rejected via destination conflict).

## Requirements *(mandatory)*

### Functional Requirements

#### Layout recognition

- **FR-001**: System MUST recognize the canonical source repository layout: services live at `services/<svc-id>/` with payload-kind subdirectories `quadlet/`, `systemd/`, and `config/`; hosts live at `hosts/<host-id>/` with `host.yaml` and per-service overlay subdirectories of the same shape.
- **FR-002**: System MUST treat each service's `config/` directory as deploying to `/etc/<config-root>/`, where `<config-root>` is taken from `services/<svc-id>/service.yaml` if that file exists, otherwise defaults to `<svc-id>`. The `config/` tree MAY contain subdirectories; a file at `services/<svc-id>/config/<rel>` deploys to `/etc/<config-root>/<rel>` preserving the relative path. Path-traversal segments (`..`) and absolute paths are rejected per FR-010.
- **FR-003**: System MUST treat each service's `quadlet/` directory as deploying to the host Quadlet directory (`/etc/containers/systemd/`), preserving file kinds (`*.container`, `*.volume`, `*.network`, `*.pod`) and drop-in shape `<unit>.<ext>.d/<file>.conf`.
- **FR-004**: System MUST treat each service's `systemd/` directory as deploying to the native systemd unit directory (`/etc/systemd/system/`), preserving file kinds (`*.socket`, `*.timer`, `*.target`, `*.mount`, `*.path`) and drop-in shape.
- **FR-005**: System MUST permit a host overlay to contribute drop-ins under `<unit>.<ext>.d/` and whole-file replacements under `config/`, and MUST reject host overlays that introduce base unit files (any file directly under `hosts/<host-id>/<svc-id>/quadlet/` or `hosts/<host-id>/<svc-id>/systemd/` that is not nested inside a `<unit>.<ext>.d/` subdirectory).

#### Schema

- **FR-006**: The `service.yaml` schema MUST consist of exactly one optional key in this revision: `config-root` (string). Any other key MUST cause the loader to reject the file with a diagnostic naming the offending key.
- **FR-007**: All YAML keys defined or introduced by this spec MUST be kebab-case. Rust deserialization MUST apply `#[serde(rename_all = "kebab-case")]` on the corresponding structures.
- **FR-008**: The `host.yaml` schema MUST continue to consist of `host` (string, host identifier) and `services` (ordered list of service identifiers). Unknown keys MUST be rejected with a diagnostic naming the offending key.

#### Validation

- **FR-009**: System MUST reject service identifiers, host identifiers, and payload-kind directory names whose first character is `_` or `.`.
- **FR-010**: System MUST reject any `config/` file whose normalized destination path escapes `/etc/<config-root>/` (e.g. through `..` segments).
- **FR-011**: System MUST reject any source repository in which two distinct files (across services and host overlays) compute to the same final destination path.
- **FR-012**: System MUST refuse to load any source repository containing a top-level `quadlets/` directory or any `services/<svc>/quadlet-overrides/` directory. The diagnostic MUST name the offending path and reference the migration guidance.
- **FR-013**: System MUST detect and reject orphan drop-ins — a drop-in at `<unit>.<ext>.d/<file>.conf` whose parent unit `<unit>.<ext>` does not exist anywhere in the merged set for the host being planned.

#### Determinism

- **FR-014**: System MUST sort base service drop-ins lexicographically by filename, then apply host drop-ins lexicographically by filename, mirroring systemd's own override order.
- **FR-015**: System MUST produce identical plan output for repeated loads of an unchanged source repository (idempotent parsing).

#### Diagnostics

- **FR-016**: When `host.yaml` selects a service whose directory does not exist, the system MUST emit a diagnostic naming both the host identifier and the missing service identifier, and pointing at the `host.yaml` source span.
- **FR-017**: When `service.yaml` is malformed, the system MUST emit a diagnostic naming the file path and the offending key or syntactic location.
- **FR-018**: When a host overlay introduces a forbidden base unit, the system MUST emit a diagnostic naming the offending file path and stating the rule.

#### CLI surface

- **FR-019**: The `core-ops` binary MUST expose a new top-level subcommand `skill` with at least one operation, `install`, and three orthogonal modes:
  - default mode writes a skill bundle to `<cwd>/.agents/skills/core-ops-source-repo/`,
  - `--global` writes the bundle to `~/.agents/skills/core-ops-source-repo/`,
  - `--print` writes the bundle to standard output and performs no filesystem writes.
- **FR-020**: The `skill install` subcommand MUST NOT modify, depend on, or be invoked by `core-ops init`. The two operations are independent.
- **FR-021**: User-facing paths produced by `core-ops` MUST NOT use vendor-specific agent directories (e.g. `.claude/skills/`). The `.agents/skills/` standard from agentskills.io is the only path the binary writes to or prints by default.

#### Skill bundle

- **FR-022**: The skill bundle MUST contain at minimum a `SKILL.md` file describing: the canonical layout, the `service.yaml` schema, the payload-kind dispatch table, the host-overlay semantics, the drop-in conventions, the validation rules from FR-009 through FR-013, and a worked authoring walk-through that produces a service the loader accepts.

#### Reference examples

- **FR-023**: Reference example source repositories MUST be provided in-tree under `specs/016-source-repository-layout/examples/`, covering at minimum:
  - a minimal single-service repository with no `service.yaml`,
  - a variant service repository with `service.yaml` declaring a non-default `config-root`,
  - a multi-unit service repository combining Quadlet containers and native systemd sockets with drop-ins on each,
  - a host overlay repository demonstrating both drop-in additions and `config/` whole-file replacements.

  > **[SUPERSEDED by spec/017]** (2026-05-05) The four in-tree example fixtures
  > `specs/016-source-repository-layout/examples/{01-minimal-single-service,02-variant-config-root,03-multi-unit-with-dropins,04-host-overlay}/`
  > are removed by spec/017's real-world-validation iteration, which publishes
  > five service-shaped examples under top-level `examples/<NN-slug>/`. The
  > same shape coverage (default config-root, variant config-root, multi-unit
  > with drop-ins, host overlay) is retained as a side effect of the
  > real-world translations. See `specs/017-real-world-validation/spec.md`
  > FR-017–FR-019 for the supersession rationale.

#### Migration

- **FR-024**: The system MUST refuse to load legacy layouts (FR-012); migration is an out-of-band operation. A migration procedure (script or documented sequence of moves) MUST be provided as part of this feature so that the live legacy repository can be converted in a single mechanical pass.
- **FR-025**: After migration, the planned host destination paths MUST be identical to the pre-migration plan; only source paths in the repository change.

### Key Entities

- **Source Repository**: the on-disk artifact containing a `services/` tree and a `hosts/` tree. The unit of input to the loader.
- **Service**: a directory at `services/<svc-id>/` containing optional `service.yaml` and one or more payload-kind subdirectories.
- **Service Identity** (`svc-id`): the directory name under `services/`. Free-form within the reserved-name rule (FR-009). Identity is independent of deployment target.
- **Config Root**: the `/etc/<root>/` directory a service's `config/` deploys to. Resolved from `service.yaml.config-root`, defaults to `svc-id`.
- **Payload Kind**: a recognized child directory of a service or host overlay. Initial set: `quadlet`, `systemd`, `config`. Each maps to a known target root. Extensible in future specs.
- **Host**: a directory at `hosts/<host-id>/` containing `host.yaml` and per-service overlay subdirectories.
- **Host Selection**: the `services` list in `host.yaml`. The host applies (and only applies) those services in the order given.
- **Host Overlay**: a per-service subdirectory at `hosts/<host-id>/<svc-id>/` that mirrors the service shape, restricted to drop-ins and `config/` replacements.
- **Drop-in**: a file at `<unit>.<ext>.d/<filename>.conf` under `quadlet/` or `systemd/` in either base service or host overlay. Merged in lexicographic order, base then host.
- **Skill Bundle**: the `core-ops-source-repo/` directory written by `core-ops skill install`. A `SKILL.md` plus any supporting assets.
- **Reserved Name**: any identifier or directory name starting with `_` or `.`. Forbidden at every root level.

## Verification Guidance *(mandatory for features that participate in the verification workflow)*

### Observable Behaviors

- After `core-ops apply`, host destination files match the dispatch table for every service and host overlay accepted by the loader.
- `core-ops plan` output for an unchanged source repository is identical across runs.
- `core-ops skill install` produces a bundle at the documented path; absent failure, no other filesystem state is modified.
- `core-ops` rejects every legacy-layout artifact named in FR-012 with a non-zero exit and a diagnostic referencing migration.

### Invariants

- A service identifier never appears in a deployed file path unless it equals its `config-root`. (Variants do not leak into `/etc/`.)
- Drop-in merge order is base-service-lex then host-overlay-lex.
- No two source files compute to the same host destination path. The loader detects and refuses any conflict.
- Reserved-name rules apply uniformly to service identifiers, host identifiers, and payload-kind directory names.

### Idempotency Expectations

- Re-running `plan` against an unchanged source repository produces an empty action set.
- Re-running `apply` against a host already in the desired state is a no-op.
- Re-running `core-ops skill install` against an already-installed bundle either writes byte-identical files or refuses with a clear diagnostic; no mid-state corruption.

### Failure Modes

- Missing service directory for a host-selected service → fatal load error pointing at `host.yaml`.
- Malformed `service.yaml` (invalid YAML, type mismatch, unknown key) → fatal load error pointing at the file and the offending key.
- Reserved-name violation (identifier or directory name) → fatal load error naming the offender and the rule.
- Host overlay introduces a base unit file → fatal load error naming the offending path.
- Destination conflict (two source files → one host path) → fatal load error naming both source files and the destination.
- Legacy-layout artifact present → fatal load error naming the path and pointing at migration guidance.
- Orphan drop-in (no parent unit in the merged set) → fatal load error naming the drop-in path.

### Upgrade Considerations

- Across CoreOps revisions that both speak the formalized layout, no source-repo action is required.
- Across the legacy-to-formalized transition (the major bump introduced by this feature), the source repository MUST be migrated before the new binary is run against it. The new binary refuses legacy layouts unconditionally.
- The `service.yaml` schema is intentionally strict about unknown keys so that future additions are explicit, opt-in, and detectable; future revisions adding keys MUST list them in the schema before the loader will accept them.

### Required Scenario Classes

- Layout conformance: minimal, variant, multi-unit, host-overlay examples each load and plan as documented.
- Host-overlay merge: drop-ins from base service and host concatenate in lex order matching systemd.
- Drop-in order: deterministic across runs and matches the documented rule.
- Conflict detection: synthetic two-source-one-destination cases are rejected.
- Reserved-name rejection: synthetic `_foo`, `.foo`, and `_quadlet` directories are rejected.
- Missing-service diagnostic: host selecting a non-existent service produces the documented error.
- Malformed-`service.yaml` diagnostic: each schema violation class produces the documented error.
- Legacy-layout rejection: every legacy artifact named in FR-012 produces the documented error.
- Skill-install round-trip: install (default, `--global`, `--print`); the bundle's `SKILL.md` is byte-identical across modes.
- Skill-driven authoring: an external agent given only the bundle authors a conformant service against each example shape.
- Migration: legacy `~/code/ulthar/repo/` after migration produces a plan whose destination set matches the pre-migration plan.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Layout parsing is pure. The loader returns a `DesiredState` (and structured errors) without performing any filesystem mutation. Side effects remain in the apply path.
- **Declarative state model**: `DesiredState` is updated to drop legacy fields, introduce per-payload-kind dispatch, and carry the resolved `config-root` per service. Plan and outcome data structures continue to reference the same `DesiredState`.
- **Idempotence & convergence**: The loader is deterministic; the planner re-derives the same plan from the same source. Apply remains idempotent (no-op when host matches plan).
- **Explicit effects/failures**: Every load failure is a `miette` diagnostic with the source span pointing at the offending file or YAML key. The hard cut on legacy layouts is loud, not silent.
- **Observability**: `plan` output reports source-repo path, host id, the resolved service set, and a content hash per service tree. Diagnostics are surfaced verbatim. Drop-in merge order is reproducible and auditable.
- **Provenance & traceability**: The status snapshot includes the source-repo revision and a `layout-version` field set to `"1"` (the formalized layout introduced by this spec) so that future revisions can detect which layout produced a given snapshot.
- **Safe defaults**: Reserved-name and conflict rules are loud-fail. The hard cut prevents silent destination drift across the legacy/formalized boundary.
- **Compatibility**: This feature is a major bump. Legacy parsing is removed; pre-existing repositories must migrate. The migration is mechanical and is delivered with this feature.
- **Release version policy**: 016 is `major` per `AGENTS.md` (deleted/renamed source plus removed CLI surface). The released CLI gains a top-level `skill` subcommand.
- **Release intent artifact**: `changes/016-source-repository-layout.md` declares `release_intent: major` and enumerates the layout change, the legacy-parser removal, the new `skill` subcommand, the bundled examples, and the migration requirement.
- **Changelog discipline**: The `[Unreleased]` block in `CHANGELOG.md` (machine-managed) records the layout formalization, the `skill install` subcommand, the in-tree examples, and the legacy-layout migration boundary.
- **Test contract**: New unit tests for the parser cover every validation rule (FR-009 to FR-013, FR-016 to FR-018). New integration tests use the in-tree examples (FR-023) and assert plan-output stability and rejection of the corrupted variants. `cargo test` and `cargo clippy --all-targets -- -D warnings` are green before merge.
- **Regenerability**: The in-tree examples (FR-023) are the canonical fixtures for the formalized layout. Spec, examples, and the skill bundle together permit reconstruction of the parser surface and authoring guidance from scratch.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An operator unfamiliar with CoreOps internals can author a conformant new service in their source repository in under 15 minutes given only this spec and one reference example, with no source-code reading required.
- **SC-002**: An external agent equipped only with the installed skill bundle produces a service definition the loader accepts on first attempt in at least 90% of trials across the four example shapes (minimal, variant, multi-unit, host-overlay).
- **SC-003**: Migration of the live legacy source repository produces a plan whose set of host destination paths matches the pre-migration plan exactly. Every service whose pre-migration destination differed from its identifier emerges with a `service.yaml` that preserves the destination.
- **SC-004**: The loader rejects 100% of synthetically corrupted layouts (each example mutated to violate exactly one validation rule) with a diagnostic that names the offending file or key.
- **SC-005**: Re-running `core-ops plan` against an unchanged source repository produces an empty action set in 100% of trials.
- **SC-006**: `core-ops skill install`, `core-ops skill install --global`, and `core-ops skill install --print` produce byte-identical `SKILL.md` content across modes.

## Assumptions

- There is exactly one live source repository in scope for the migration described in User Story 3 (`~/code/ulthar/repo/`). The migration tooling does not need to be a general-purpose library.
- The initial payload-kind set (`quadlet`, `systemd`, `config`) is sufficient for current services. Future kinds (`cni`, `init`, ...) are deferred to subsequent specs that follow the same pattern.
- Whole-file replacement remains the only semantic for `config/` overlays in this revision. Templating, partial-file merging, and absolute-path escape hatches are explicitly deferred.
- The agentskills.io standard at `.agents/skills/<skill-name>/` is stable and appropriate as the default user-facing skill path. If that standard changes, the default path updates with it.
- `core-ops init` continues to govern controller initialization only; it does not gain knowledge of the skill bundle or the source-repo layout in this spec.
