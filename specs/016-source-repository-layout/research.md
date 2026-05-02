# Research: Source Repository Layout Formalization

**Feature**: 016-source-repository-layout
**Date**: 2026-05-01
**Status**: Decisions locked. No open NEEDS CLARIFICATION items.

This document records the design decisions reached during planning. Each decision states what was chosen, why, and which alternatives were rejected and on what evidence.

---

## D1 — Hard cut vs. deprecation window

**Decision**: Hard cut. The new binary refuses to load any source repository containing legacy artifacts (`quadlets/` at root, `services/<svc>/quadlet-overrides/`, `hosts/<h>/overrides/`). The feature ships as a major version bump (`Cargo.toml` 1.0.0 → 2.0.0).

**Rationale**: There is exactly one source repository today (`~/code/ulthar/repo/`), under the user's sole control. Mechanical migration is bounded and recoverable. A deprecation window would carry the legacy parser as dead weight, increase test surface, and create two correct shapes during the window — exactly the ambiguity this feature exists to remove.

**Alternatives rejected**:

- *Deprecation window* (warn on legacy, remove later): adds carry-cost (two parsers, two test trees) for no user benefit at this scale. Recommended posture if there were multiple downstream repos; not justified here.
- *Coexistence* (legacy stays grandfathered): would defeat the formalization (FR-001) by leaving two valid shapes in perpetuity. Rejected.

---

## D2 — Variant services and the `service.yaml`

**Decision**: A service may declare a `config-root` distinct from its identifier in an optional `service.yaml`. When the file is absent, `config-root` defaults to the service identifier. The schema for v1 contains exactly one optional key (`config-root`); unknown keys are rejected.

**Rationale**: Concrete evidence in the live repository: `services/traefik-dnschallenge/config/etc/traefik/traefik.toml` deploys to `/etc/traefik/`, not `/etc/traefik-dnschallenge/`. A naive identifier-equals-destination rule would silently relocate the file on the host and break the service. Explicit declaration is the only correct rule.

**Alternatives rejected**:

- *Convention strip* (`<svc>-<variant>` → strip `-…` to get the root): implicit, fragile, undocumented. Two services like `traefik` and `traefik-dnschallenge` could not coexist if the latter were to use `traefik-dnschallenge` as a literal config root. Rejected.
- *Manifest-required for every service*: imposes boilerplate on the common case where the identifier already equals the desired root (most services in the live repo). Rejected as ergonomically wasteful.
- *Inline annotation in directory name* (`services/traefik-dnschallenge[traefik]/`): non-portable shell behavior, confuses many tools. Rejected.

---

## D3 — Payload-kind directories at service root

**Decision**: Each service directory contains zero or more payload-kind subdirectories. The initial set is `quadlet/`, `systemd/`, and `config/`, each with a known target dispatch root. The set is extensible — future kinds (e.g. `cni/`, `init/`) follow the same pattern in subsequent specs.

**Rationale**: A directory-per-payload-kind keeps the service tree self-describing and enables additive growth without parser branching. Each kind maps 1:1 to a known target directory on the host, so the dispatch is a single switch. Drop-ins live inside the same payload directory as their parent unit, mirroring systemd's own filesystem.

**Dispatch table**:

| Source path | Target on host | Generator |
|---|---|---|
| `services/<svc>/quadlet/<unit>.<ext>` | `/etc/containers/systemd/<unit>.<ext>` | `podman-system-generator` (compiled at boot/reload) |
| `services/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` | `/etc/containers/systemd/<unit>.<ext>.d/<file>.conf` | (same) |
| `services/<svc>/systemd/<unit>.<ext>` | `/etc/systemd/system/<unit>.<ext>` | none (native unit) |
| `services/<svc>/systemd/<unit>.<ext>.d/<file>.conf` | `/etc/systemd/system/<unit>.<ext>.d/<file>.conf` | none |
| `services/<svc>/config/<file>` | `/etc/<config-root>/<file>` | none (whole-file copy) |

**Alternatives rejected**:

- *Flat service directory* (units and config files at service root, no payload-kind dir): mixes file kinds with different validation grammars and target roots. Rejected: a `*.toml` config and a `*.container` Quadlet at the same level lose their semantic separation.
- *Single `units/` directory for everything systemd-shaped*: collapses Quadlet and native systemd. Rejected — see D5.

---

## D4 — Host overlay shape

**Decision**: A host overlay mirrors the service tree directly: `hosts/<host-id>/<svc-id>/{quadlet,systemd,config}/...`. The legacy `overrides/` directory is removed. A host overlay can contribute drop-ins (`<unit>.<ext>.d/<file>.conf`) and `config/` whole-file replacements; it cannot introduce base unit files.

**Rationale**: Symmetry with the service tree means an operator who knows how to author a service directly knows how to author a host overlay. The `overrides/` segment was an asymmetric naming convention (`quadlet/` in services vs `overrides/quadlet/` in hosts) with no semantic content beyond marking the parent as a host. Eliminating it removes a level of nesting and a vocabulary.

**Alternatives rejected**:

- *Keep `overrides/`*: documented churn for no semantic benefit. Rejected.
- *Allow base units in host overlay*: would let a host redefine a service entirely. That is a different feature ("host-private services"); deferred to a future spec if ever needed.
- *Add a `services/` segment under host* (`hosts/<h>/services/<svc>/`): extra path component for no disambiguation benefit. The host root already has only `host.yaml` plus per-service overlay directories.

---

## D5 — Drop-in shape

**Decision**: Drop-ins use systemd's native filesystem shape verbatim: `<unit>.<ext>.d/<file>.conf`. Drop-ins are siblings of their parent unit inside the same payload-kind directory.

**Rationale**: An operator running `systemctl cat <unit>` sees overrides in this exact shape. `systemctl edit <unit>` writes to this exact path. Diverging in the source repository creates a mental translation layer that costs more than the six characters of nesting it would save. Multi-unit services (the live `traefik-dnschallenge` has six units) cannot disambiguate drop-ins under a flat `overrides/` directory without filename overloading.

**Alternatives rejected**:

- *Flat `overrides/<file>.conf` per service*: cannot bind a drop-in to a specific unit when a service has multiple units. Rejected with concrete counter-evidence (traefik has `*.container`, `*.network`, `*.volume`, `*.socket` simultaneously eligible for drop-ins).
- *Filename overloading* (`<unit>.<ext>.<filename>.conf`): hard to read; conflates two dimensions of identity into one filename. Rejected.
- *YAML/TOML manifest declaring drop-in targets*: invasive, breaks parity with `systemctl cat`. Rejected.

---

## D6 — Quadlet vs systemd split

**Decision**: Two payload-kind directories: `quadlet/` for files consumed by the Quadlet generator (`*.container`, `*.volume`, `*.network`, `*.pod`), and `systemd/` for native systemd units (`*.socket`, `*.timer`, `*.target`, `*.mount`, `*.path`).

**Rationale**: Quadlet files are *not* unit files; they are inputs to `podman-system-generator` which compiles them into runtime units in `/run/systemd/generator/`. Native systemd files are unit files installed verbatim to `/etc/systemd/system/`. They have different generators, different validation grammars, and different reload semantics. Lumping them under a single name was the existing source of confusion.

The legacy `quadlet/` directory in `~/code/ulthar/repo/services/traefik-dnschallenge/` already contains both Quadlet inputs (`*.container`, `*.volume`, `*.network`) and native systemd files (`*.socket`) — proof that the misnomer is misleading in practice.

**Alternatives rejected**:

- *Single `units/` directory*: hides the generator vs. native distinction. Rejected.
- *Keep the misleading `quadlet/` for everything*: violates the constitution's "open standards and native interfaces" principle. Rejected.

---

## D7 — Reserved name namespace

**Decision**: Identifiers and directory names beginning with `_` or `.` are reserved for future metadata at every root level (`services/<id>/`, `hosts/<id>/`, repository root). The loader rejects them.

**Rationale**: Reserves a namespace before it is needed, costing nothing today (no service or host today starts with these characters) and preventing future foot-guns when metadata files (`_lock`, `.git/` already excluded by being a hidden dotdir, etc.) need to coexist with content directories.

**Alternatives rejected**:

- *Reserve specific names only* (e.g. just `service.yaml`, `host.yaml`): reactive; the next reserved name forces a breaking change.
- *No reservation*: defers the problem.

---

## D8 — Skill distribution

**Decision**: A new top-level CLI subcommand `core-ops skill` with one operation `install` and three orthogonal modes:

- default: writes the skill bundle to `<cwd>/.agents/skills/core-ops-source-repo/`.
- `--global`: writes the bundle to `~/.agents/skills/core-ops-source-repo/`.
- `--print`: writes the bundle (concatenated or as a tar stream) to standard output and performs no filesystem writes.

The path standard is **agentskills.io** (`.agents/skills/<skill-name>/`), not Claude-specific paths (`.claude/skills/`).

**Rationale**: CoreOps is a paid tool and is not a vendor-promotion platform. The agentskills.io standard is vendor-neutral, documented, and addresses Anthropic's Claude, OpenAI's Codex, and other agent runtimes uniformly. The `core-ops` binary writes to vendor-neutral paths by default; users who want a vendor-specific path can copy the bundle themselves. The print/install symmetry mirrors the established `clap_complete` `completions <shell>` pattern.

This subcommand is independent of `core-ops init`. `init` initializes a controller against a source repository; the skill subcommand installs an authoring aid. Conflating them was rejected by the user.

**Alternatives rejected**:

- *Default to `.claude/skills/`*: vendor-locks CoreOps onto Anthropic. Rejected explicitly by the user.
- *Wire skill install into `core-ops init`*: conflates two unrelated operations. Rejected.
- *Distribute the skill as a separate package*: increases the surface (a second crate or release artifact) for no benefit. The bundle is small enough to embed in the binary.

---

## D9 — YAML key style

**Decision**: All new YAML keys introduced by this spec (in `service.yaml`, future schemas) use kebab-case. Rust deserialization applies `#[serde(rename_all = "kebab-case")]` to the corresponding structs.

**Rationale**: Consistent with the surrounding Linux ecosystem (systemd `[Section]` keys are camel-style by their own convention, but YAML in the kubectl/Kubernetes/Helm/agentskills.io world is kebab-case). The user has stated this preference explicitly.

The existing snake_case key in `changes/<id>.md` frontmatter (`release_intent`) is grandfathered — not retroactively renamed.

**Alternatives rejected**:

- *Snake case (`config_root`)*: breaks ecosystem convention.
- *Camel case (`configRoot`)*: foreign to YAML.

---

## D10 — Migration approach

**Decision**: Migration of the live legacy source repository (`~/code/ulthar/repo/`) is performed by a one-off shell script delivered with this feature at `scripts/migrate-legacy-source-repo.sh`. The script is idempotent (running it on an already-migrated repo is a no-op) and operates by file moves only — no semantic re-interpretation. After running, `core-ops plan` against the migrated repository produces a destination-set identical to the pre-migration plan (SC-003).

**Rationale**: The migration is a single, bounded, recoverable operation against one known repository. Building a general-purpose migration command into the binary would carry permanent code surface for a one-off concern. A shell script that the user runs once meets the requirement and disappears afterward (or stays available for posterity, for the cost of one file).

The migration mapping is mechanical:

| Legacy path | New path |
|---|---|
| `services/<svc>/quadlet/<unit>.<ext>` | (unchanged) |
| `services/<svc>/quadlet/<unit>.socket` | `services/<svc>/systemd/<unit>.socket` (kind reassignment) |
| `services/<svc>/quadlet-overrides/<unit>.<ext>.d/<file>.conf` | `services/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/config/etc/<svc>/<file>` | `services/<svc>/config/<file>` |
| `services/<svc>/config/etc/<other>/<file>` | `services/<svc>/config/<file>` + `services/<svc>/service.yaml` declaring `config-root: <other>` |
| `hosts/<h>/overrides/quadlet/<unit>.<ext>.d/<file>` | `hosts/<h>/<svc-id>/quadlet/<unit>.<ext>.d/<file>` — `<svc-id>` is resolved by building a unit→service ownership map from `services/<svc>/quadlet/` and `services/<svc>/systemd/` before migration; if a host drop-in references a unit not owned by exactly one service, the script fails loudly naming the offending unit |
| `hosts/<h>/overrides/config/etc/<svc>/<file>` | `hosts/<h>/<svc>/config/<file>` |

**Alternatives rejected**:

- *General-purpose migration subcommand in `core-ops`*: maintenance cost outweighs benefit for one user, one repository.
- *Manual migration documented step-by-step*: error-prone; a script is the right shape.

---

## Summary of locked decisions

All 10 decisions above were resolved during planning conversations and are encoded in the spec (`spec.md`) as functional requirements. No `[NEEDS CLARIFICATION]` markers remain. The plan and downstream artifacts (data-model, contracts, quickstart) consume these decisions without re-litigation.
