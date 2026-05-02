---
name: core-ops-source-repo
description: Author a CoreOps source repository — services, host overlays, and drop-ins — that the `core-ops` loader accepts on first attempt
layout-version: "1"
---

# core-ops Source Repository — Authoring Guide

You are authoring a **CoreOps source repository**: an on-disk tree that the `core-ops` controller loads, plans against, and applies to a fleet of hosts. This skill is the canonical reference for the layout. The loader's behavior is what this document specifies; if the loader rejects a tree this skill says is valid, the skill (or the spec) is wrong, not the loader.

The skill is self-contained. You do not need access to the `core-ops` source tree to use it.

> **Layout version**: `1`. The status snapshot the controller persists records this version under `desired_state.layout-version`. Future spec revisions that change the on-disk shape will bump this number.

---

## 1. Repository shape

Every CoreOps source repository has exactly two top-level content directories:

```
<repo-root>/
├── services/      # what to deploy
└── hosts/         # which host gets what
```

Anything else at the top level is tolerated **only** if its name begins with `_` or `.` (the reserved namespace — admits `.git/`, `_local/`, `.gitignore`, `README.md` is also tolerated as a regular file, etc.). A top-level `quadlets/` directory is **rejected** as a legacy artifact (see §10).

### `services/<svc-id>/`

A service is a directory under `services/`. The directory name is the **service identifier** (`<svc-id>`).

```
services/<svc-id>/
├── service.yaml          # OPTIONAL — see §3
├── quadlet/              # Quadlet inputs (containers, volumes, networks, pods)
│   ├── <unit>.container
│   ├── <unit>.volume
│   ├── <unit>.network
│   ├── <unit>.pod
│   └── <unit>.<ext>.d/
│       └── <file>.conf   # drop-ins for the unit one level up
├── systemd/              # Native systemd units (sockets, timers, mounts, …)
│   ├── <unit>.socket
│   ├── <unit>.timer
│   ├── <unit>.target
│   ├── <unit>.mount
│   ├── <unit>.path
│   └── <unit>.<ext>.d/
│       └── <file>.conf
└── config/               # Whole-file configs deployed to /etc/<config-root>/
    └── <relative-path>
```

All three payload-kind subdirectories (`quadlet/`, `systemd/`, `config/`) are optional individually; at least one must contain content for the service to be useful. **Files at the service root are forbidden** — a `*.container` directly under `services/<svc-id>/` (the legacy single-flat shape) is rejected.

### `hosts/<host-id>/`

A host is a directory under `hosts/`. The directory name is the **host identifier** (`<host-id>`), which the controller resolves at runtime via the `CORE_OPS_HOST` environment variable or the local hostname.

```
hosts/<host-id>/
├── host.yaml             # REQUIRED — see §4
└── <svc-id>/             # OPTIONAL host overlay for this service
    ├── quadlet/
    │   └── <unit>.<ext>.d/<file>.conf   # drop-ins ONLY — no base units
    ├── systemd/
    │   └── <unit>.<ext>.d/<file>.conf   # drop-ins ONLY
    └── config/
        └── <relative-path>              # whole-file overrides
```

The host overlay tree mirrors the service tree directly under `hosts/<host-id>/<svc-id>/`. There is **no `overrides/` segment** between them — that's a legacy artifact and is rejected.

---

## 2. Identifier rules

`<svc-id>` and `<host-id>` are free-form names with two constraints:

1. They match `^[A-Za-z0-9][A-Za-z0-9._-]*$` (alphanumeric, dot, underscore, hyphen; first character must be alphanumeric).
2. The first character **MUST NOT** be `_` or `.`. These prefixes are reserved for future metadata.

A service or host directory whose name begins with `_` or `.` is rejected with a diagnostic naming the offending entry. (`.git/`, `_local/` etc. are tolerated at the *top level* of the repository, but not as service/host identifiers.)

The same identifier rule applies to `<config-root>` (see §3).

Payload-kind directory names are exactly `quadlet`, `systemd`, `config`. Other names at this level are rejected.

---

## 3. `service.yaml`

Optional. When present, it declares the service's `<config-root>`:

```yaml
config-root: traefik
```

| Key | Type | Required | Default | Notes |
|---|---|---|---|---|
| `config-root` | string | yes | — | The directory under `/etc/` where this service's `config/` payload is deployed. |

If `service.yaml` is absent, `<config-root>` defaults to `<svc-id>` — i.e. a service named `whoami` deploys its `config/whoami.toml` to `/etc/whoami/whoami.toml`.

If `service.yaml` declares `config-root: traefik` for a service named `traefik-dnschallenge`, its `config/traefik.yaml` deploys to `/etc/traefik/traefik.yaml`.

**Strict deserialization**: unknown keys, missing required keys, and any YAML parse error produce a diagnostic naming the file path and the offending line/key. There is no permissive fallback.

**Style**: keys are kebab-case (`config-root`, not `config_root`).

---

## 4. `host.yaml`

Required. Declares the host's identity and the services it selects:

```yaml
host: kadath
services:
  - traefik
  - immich
  - vector
```

| Key | Type | Required | Notes |
|---|---|---|---|
| `host` | string | yes | MUST match the directory name (`hosts/<host-id>/host.yaml` → `host: <host-id>`) |
| `services` | list of strings | yes | Each entry MUST be a `<svc-id>` that exists under `services/` |

A `services` entry naming a non-existent service produces a diagnostic identifying both the host and the missing service id, pointing at the `host.yaml` source span.

The `services` list is **not** a default — it is the exact set the host applies. Services not listed are not deployed to that host.

**Strict deserialization**: same rule as `service.yaml`. Unknown keys are rejected.

---

## 5. Payload-kind dispatch table

The loader knows three payload kinds. Each maps a source path to a destination on the host:

| Source path | Target on host |
|---|---|
| `services/<svc>/quadlet/<unit>.<ext>` | `/etc/containers/systemd/<unit>.<ext>` |
| `services/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` | `/etc/containers/systemd/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/systemd/<unit>.<ext>` | `/etc/systemd/system/<unit>.<ext>` |
| `services/<svc>/systemd/<unit>.<ext>.d/<file>.conf` | `/etc/systemd/system/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/config/<rel>` | `/etc/<config-root>/<rel>` |
| `hosts/<h>/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` | merged after base service drop-ins, lex-sorted by filename |
| `hosts/<h>/<svc>/systemd/<unit>.<ext>.d/<file>.conf` | (same) |
| `hosts/<h>/<svc>/config/<rel>` | replaces `/etc/<config-root>/<rel>` whole-file |

### Allowed unit extensions

| Payload kind | Extensions |
|---|---|
| `quadlet` | `.container`, `.volume`, `.network`, `.pod` |
| `systemd` | `.socket`, `.timer`, `.target`, `.mount`, `.path` |

A unit in the wrong payload-kind directory (e.g. `quadlet/foo.socket`) is rejected with a diagnostic naming the offending file and the expected payload kind. The split is structural, not stylistic — `quadlet/` is for Quadlet-generator inputs that podman-systemd compiles into runtime units; `systemd/` is for native unit files that systemd loads directly.

---

## 6. Host overlay semantics

A host overlay at `hosts/<host-id>/<svc-id>/` contributes two kinds of additions to a service the host has selected:

### 6.a Drop-ins

Place `<file>.conf` at:

```
hosts/<host-id>/<svc-id>/quadlet/<unit>.<ext>.d/<file>.conf
hosts/<host-id>/<svc-id>/systemd/<unit>.<ext>.d/<file>.conf
```

The parent unit (`<unit>.<ext>`) MUST exist somewhere in the merged set — either declared by the base service or contributed by this overlay's drop-ins. If no parent exists anywhere, the overlay is rejected as an orphan drop-in.

**Base units in host overlays are forbidden.** Placing a `*.container` or `*.socket` directly under `hosts/<h>/<svc>/quadlet/` (no `.d/` directory) is rejected with a diagnostic stating the rule. If a host needs to redefine a base unit, it must do so in the service that owns the unit, not as a host-specific overlay.

### 6.b Whole-file `config/` replacements

Place a file at:

```
hosts/<host-id>/<svc-id>/config/<relative-path>
```

The host's file replaces the base service's file at the same `<relative-path>` byte-for-byte at the destination `/etc/<config-root>/<relative-path>`. There is no merge semantics for config files — host wins, full file.

### 6.c What overlays can NOT do

- Cannot rename a base unit (mount different content under a different name)
- Cannot delete a base unit from a specific host (selection is the only opt-out — if a host shouldn't deploy a service, omit the service from `host.yaml`'s `services` list)
- Cannot introduce new units (only drop-ins) — a host wanting an entirely host-specific service should declare it as a normal service under `services/` and select it only from that host's `host.yaml`

---

## 7. Drop-in conventions

Drop-ins are systemd-native: a `<unit>.<ext>.d/<file>.conf` directory next to (or "for") a base unit, containing one or more `*.conf` files that augment the base unit at load time.

### Order

1. Base service drop-ins applied first, sorted lexicographically by `<file>.conf`.
2. Host overlay drop-ins applied second, sorted lexicographically.

The conventional `NN-name.conf` pattern (e.g. `10-defaults.conf`, `20-host-overrides.conf`, `30-listen-port.conf`) makes the order visible at a glance and is strongly recommended.

### Content

Each drop-in is a partial systemd unit file with `[Section]` headers and key-value entries. Only `.conf` files are recognized; other extensions in a `*.d/` directory are rejected.

### Drop-ins on which units?

Any base unit declared in either `quadlet/` or `systemd/` may have drop-ins:

| Base unit lives in | Drop-in directory lives in |
|---|---|
| `services/<svc>/quadlet/foo.container` | `services/<svc>/quadlet/foo.container.d/` (and/or `hosts/<h>/<svc>/quadlet/foo.container.d/`) |
| `services/<svc>/systemd/foo.socket` | `services/<svc>/systemd/foo.socket.d/` (and/or `hosts/<h>/<svc>/systemd/foo.socket.d/`) |

The drop-in directory MUST live in the same payload-kind subtree as its parent unit. A drop-in for a `*.socket` does not go in `quadlet/`.

---

## 8. Validation rules (the loader rejects each of these)

| Rule | Trigger | Diagnostic |
|---|---|---|
| **Reserved name** | service id, host id, or payload-kind dir name begins with `_` or `.` | "reserved name '<name>' (must not begin with '_' or '.')" |
| **Config path traversal** | a `config/` file's normalized destination escapes `/etc/<config-root>/` | "config file destination escapes /etc/<root>/: <path>" |
| **Destination conflict** | two distinct files compute to the same destination path | "destination conflict at <target>: <a> and <b>" |
| **Legacy artifact** | top-level `quadlets/`, any `services/<svc>/quadlet-overrides/`, or any `hosts/<h>/overrides/` | "legacy layout artifact: <path>" with a pointer to the migration script |
| **Orphan drop-in** | a drop-in whose parent unit does not exist anywhere in the merged set | "orphan drop-in at <path> (no matching unit '<unit>')" |

Plus the per-key rules covered in §3, §4, and §6.

---

## 9. Determinism

For any two runs against an unchanged repository, the loader produces a byte-identical desired state:

- Services are processed in lex order of `<svc-id>`.
- Within a payload tree, base units are processed in lex order of their full filename.
- Drop-ins for a given parent unit are applied in lex order of `<file>.conf`, base service first, then host overlay.
- Config files are processed in lex order of `<relative-path>`.

Any non-determinism observed at the loader level is a bug in the loader, not the repository.

---

## 10. What's no longer accepted (legacy artifacts)

The loader is a **hard cut** from the pre-spec-016 layout. None of these are accepted:

| Legacy path | Replacement under v1 |
|---|---|
| `<repo-root>/quadlets/<unit>.<ext>` | `services/<svc>/quadlet/<unit>.<ext>` |
| `services/<svc>/quadlet-overrides/<unit>.<ext>.d/<file>.conf` | `services/<svc>/quadlet/<unit>.<ext>.d/<file>.conf` |
| `hosts/<h>/overrides/<unit>.<ext>.d/<file>.conf` | `hosts/<h>/<svc>/{quadlet,systemd}/<unit>.<ext>.d/<file>.conf` |
| `services/<svc>/config/etc/<config-root>/<rel>` | `services/<svc>/config/<rel>` |
| `services/<svc>/<unit>.<ext>` (file at service root) | `services/<svc>/{quadlet,systemd}/<unit>.<ext>` |

The diagnostic for any legacy artifact references `scripts/migrate-legacy-source-repo.sh`, the operator-facing tool that converts a legacy repository in one mechanical pass. (The migration is a one-time per-repo operation; spec 016 ships with that script.)

---

## 11. Worked walk-throughs (the four canonical shapes)

These are the four fixture shapes the loader's contract tests exercise. Each is a complete repository — copy any one verbatim, `git init && git add -A && git commit`, and `core-ops plan` succeeds against it (with `CORE_OPS_HOST` set to the example's host id).

The fixtures live at `specs/016-source-repository-layout/examples/<NN>-<shape>/` in the `core-ops` repository. The trees below are inlined so you can author from this skill alone.

### 11.a Minimal single service — `01-minimal-single-service`

The simplest possible repository: one service, one container, one config file, one host. No `service.yaml` (so `<config-root>` defaults to `whoami`).

```
01-minimal-single-service/
├── README.md
├── services/
│   └── whoami/
│       ├── quadlet/
│       │   └── whoami.container
│       └── config/
│           └── whoami.toml
└── hosts/
    └── example-host/
        └── host.yaml
```

`services/whoami/quadlet/whoami.container`:

```
[Unit]
Description=whoami HTTP service

[Container]
Image=docker.io/traefik/whoami:v1.10
PublishPort=80:80

[Install]
WantedBy=default.target
```

`services/whoami/config/whoami.toml`:

```toml
# deployed to /etc/whoami/whoami.toml
greeting = "hello from CoreOps"
```

`hosts/example-host/host.yaml`:

```yaml
host: example-host
services:
  - whoami
```

Resolved destinations on the host:

- `/etc/containers/systemd/whoami.container`
- `/etc/whoami/whoami.toml`

### 11.b Variant config-root — `02-variant-config-root`

A service whose id differs from its `<config-root>`. The id is `traefik-dnschallenge` (the deployment variant); the config root is `traefik` (the upstream tool's expected directory).

```
02-variant-config-root/
├── README.md
├── services/
│   └── traefik-dnschallenge/
│       ├── service.yaml
│       ├── quadlet/
│       │   └── traefik-dnschallenge.container
│       └── config/
│           └── traefik.yaml
└── hosts/
    └── example-host/
        └── host.yaml
```

`services/traefik-dnschallenge/service.yaml`:

```yaml
config-root: traefik
```

Resolved destinations:

- `/etc/containers/systemd/traefik-dnschallenge.container`
- `/etc/traefik/traefik.yaml` ← note: `/etc/traefik/`, NOT `/etc/traefik-dnschallenge/`

This is the canonical reason `service.yaml` exists. Multiple deployment variants of the same upstream tool (e.g. `traefik-dnschallenge`, `traefik-internal`) can each declare `config-root: traefik` and share `/etc/traefik/`. Without `service.yaml`, two variants of the same tool would collide on each other's `<svc-id>` directory under `/etc/`.

### 11.c Multi-unit with drop-ins — `03-multi-unit-with-dropins`

A service with both a Quadlet container and a native systemd socket, each with a drop-in.

```
03-multi-unit-with-dropins/
├── README.md
├── services/
│   └── webhook-receiver/
│       ├── quadlet/
│       │   ├── webhook-receiver.container
│       │   └── webhook-receiver.container.d/
│       │       └── 10-resources.conf
│       └── systemd/
│           ├── webhook-receiver.socket
│           └── webhook-receiver.socket.d/
│               └── 10-hardening.conf
└── hosts/
    └── example-host/
        └── host.yaml
```

`services/webhook-receiver/quadlet/webhook-receiver.container.d/10-resources.conf`:

```
[Service]
MemoryMax=256M
CPUQuota=25%
```

`services/webhook-receiver/systemd/webhook-receiver.socket.d/10-hardening.conf`:

```
[Socket]
SocketMode=0600
```

The container goes in `quadlet/` (it's a `*.container`); the socket goes in `systemd/` (it's a native `*.socket`). Each drop-in lives next to its parent, in the same payload-kind subtree.

Resolved destinations:

- `/etc/containers/systemd/webhook-receiver.container`
- `/etc/containers/systemd/webhook-receiver.container.d/10-resources.conf`
- `/etc/systemd/system/webhook-receiver.socket`
- `/etc/systemd/system/webhook-receiver.socket.d/10-hardening.conf`

### 11.d Host overlay — `04-host-overlay`

A base service plus a host that contributes both a drop-in addition and a whole-file `config/` replacement.

```
04-host-overlay/
├── README.md
├── services/
│   └── node-exporter/
│       ├── quadlet/
│       │   └── node-exporter.container
│       └── config/
│           └── node-exporter.env
└── hosts/
    └── host-a/
        ├── host.yaml
        └── node-exporter/
            ├── quadlet/
            │   └── node-exporter.container.d/
            │       └── 30-listen-port.conf
            └── config/
                └── node-exporter.env
```

`hosts/host-a/host.yaml`:

```yaml
host: host-a
services:
  - node-exporter
```

`hosts/host-a/node-exporter/quadlet/node-exporter.container.d/30-listen-port.conf`:

```
[Container]
Environment=NODE_EXPORTER_LISTEN=:9101
```

`hosts/host-a/node-exporter/config/node-exporter.env`:

```
# host-a's specific environment file — replaces the base whole-file
NODE_EXPORTER_LOG_LEVEL=debug
```

When `core-ops plan` runs on `host-a`:

- `/etc/containers/systemd/node-exporter.container` is the base container.
- `/etc/containers/systemd/node-exporter.container.d/30-listen-port.conf` is the host's drop-in addition.
- `/etc/node-exporter/node-exporter.env` is the **host's** file, not the base service's. Whole-file replacement.

Note the absence of an `overrides/` directory between `hosts/host-a/` and `node-exporter/`. The overlay tree mirrors the service tree directly.

---

## 12. Authoring checklist

When you finish authoring a repository or a change to one, walk this list:

1. **Top-level shape**: only `services/` and `hosts/` (plus `_*` / `.*` reserved names like `.git/`, `README.md`).
2. **Identifiers**: every service id and host id matches `^[A-Za-z0-9][A-Za-z0-9._-]*$`.
3. **Payload-kind dirs**: every file under a service is in `quadlet/`, `systemd/`, or `config/` — no `*.container` directly at service root.
4. **Extensions match payload kind**: `*.container/.volume/.network/.pod` in `quadlet/`; `*.socket/.timer/.target/.mount/.path` in `systemd/`.
5. **`service.yaml`**: present iff `<config-root>` differs from `<svc-id>`. Schema: only `config-root` key, kebab-case.
6. **`host.yaml`**: present in every `hosts/<h>/`. `host` matches the directory name. Every entry in `services` has a corresponding `services/<svc-id>/` directory.
7. **Host overlays**: only drop-ins under `*.d/` and whole-file replacements under `config/`. No base units at `hosts/<h>/<svc>/{quadlet,systemd}/<unit>.<ext>` (without `.d/`).
8. **Drop-ins**: every `<unit>.<ext>.d/<file>.conf` has a corresponding base unit somewhere in the merged service+overlay set.
9. **No legacy artifacts**: no top-level `quadlets/`, no `services/<svc>/quadlet-overrides/`, no `hosts/<h>/overrides/`, no `config/etc/...` mirror.
10. **Determinism**: the same input must yield the same `core-ops plan` output. If you observe drift, that is a bug — file it.

`core-ops plan` against your repository is the authoritative test. If it succeeds and the destinations look right, the repository is well-formed.
