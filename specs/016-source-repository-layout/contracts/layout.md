# Contract: Source Repository Layout (Normative)

**Feature**: 016-source-repository-layout | **Layout version**: `1`

This document is the normative on-disk shape of a CoreOps source repository under spec 016. It is the contract between the source repository (input) and `core-ops` (consumer). Any deviation MUST cause the loader to fail with a typed error.

## Tree

```text
<repo-root>/
├── services/
│   └── <svc-id>/
│       ├── service.yaml                       # OPTIONAL; conforms to service-yaml.schema.yaml
│       ├── quadlet/
│       │   ├── <unit>.container               # base Quadlet inputs
│       │   ├── <unit>.volume
│       │   ├── <unit>.network
│       │   ├── <unit>.pod
│       │   └── <unit>.<ext>.d/
│       │       └── <file>.conf                # drop-ins for the corresponding base unit
│       ├── systemd/
│       │   ├── <unit>.socket                  # base native systemd units
│       │   ├── <unit>.timer
│       │   ├── <unit>.target
│       │   ├── <unit>.mount
│       │   ├── <unit>.path
│       │   └── <unit>.<ext>.d/
│       │       └── <file>.conf
│       └── config/
│           └── <relative-path>                # whole-file payload deployed to /etc/<config-root>/
└── hosts/
    └── <host-id>/
        ├── host.yaml                          # REQUIRED; conforms to host-yaml.schema.yaml
        └── <svc-id>/                          # OPTIONAL host overlay for this service
            ├── quadlet/
            │   └── <unit>.<ext>.d/<file>.conf # drop-ins only — base units NOT permitted in host overlay
            ├── systemd/
            │   └── <unit>.<ext>.d/<file>.conf # drop-ins only
            └── config/
                └── <relative-path>            # whole-file overlay of /etc/<config-root>/<relative-path>
```

## Identifier rules

- `<svc-id>` and `<host-id>`: free-form names matching `^[A-Za-z0-9][A-Za-z0-9._-]*$`, with the additional restriction that **the first character MUST NOT be `_` or `.`**.
- Payload-kind directory names are exactly `quadlet`, `systemd`, `config`. Other names at this level are rejected.
- Top-level directories under the repository root are exactly `services` and `hosts`. Other names are tolerated only if they begin with `_` or `.` (the reserved namespace) — this admits `.git/`, `_local/`, etc.

## Dispatch table

| Source path | Target on host |
|---|---|
| `services/<svc>/quadlet/<unit>.<ext>` | `/etc/containers/systemd/<unit>.<ext>` |
| `services/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` | `/etc/containers/systemd/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/systemd/<unit>.<ext>` | `/etc/systemd/system/<unit>.<ext>` |
| `services/<svc>/systemd/<unit>.<ext>.d/<file>.conf` | `/etc/systemd/system/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/config/<rel>` | `/etc/<config-root>/<rel>` |
| `hosts/<h>/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` | merged after base service drop-ins, lex-sorted by filename |
| `hosts/<h>/<svc>/systemd/<unit>.<ext>.d/<file>.conf` | (same) |
| `hosts/<h>/<svc>/config/<rel>` | replaces the corresponding `/etc/<config-root>/<rel>` whole-file |

`<config-root>` is read from `services/<svc>/service.yaml` if present; otherwise it defaults to `<svc>`.

## Allowed unit extensions

| Payload kind | Extensions |
|---|---|
| `quadlet` | `.container`, `.volume`, `.network`, `.pod` |
| `systemd` | `.socket`, `.timer`, `.target`, `.mount`, `.path` |

A unit in the wrong payload-kind directory (e.g. `quadlet/foo.socket`) is rejected with a diagnostic naming the offending file and the expected payload kind.

## Determinism

- Services are processed in lex order of `<svc-id>`.
- Within a payload tree, base units are processed in lex order of their full filename.
- Drop-ins for a given parent unit are applied in lex order of `<file>.conf`, base service first, then host overlay.
- Config files are processed in lex order of `<relative-path>`.

The above rules are sufficient to make `DesiredState` byte-identical for any two runs against an unchanged repository.

## Forbidden artifacts

The following paths cause a fatal `LayoutError::LegacyArtifact`:

- `<repo-root>/quadlets/` (legacy single-flat layout)
- `services/<svc>/quadlet-overrides/` (legacy split drop-ins)
- `hosts/<h>/overrides/` (legacy host overrides directory)

The diagnostic references `scripts/migrate-legacy-source-repo.sh` for resolution.
