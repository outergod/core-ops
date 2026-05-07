# CoreOps

<p align="center">
  <img src="docs/core-ops.svg" alt="CoreOps logo" width="160">
</p>

<p align="center">
  <strong>Host-native convergence for systemd-based systems</strong>
</p>

<p align="center">
<a href="https://github.com/outergod/core-ops/actions/workflows/ci.yml"><img src="https://github.com/outergod/core-ops/actions/workflows/ci.yml/badge.svg?branch=master" alt="CI"></a>
<a href="https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml"><img src="https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml/badge.svg" alt="E2E Gate"></a>
<a href="https://github.com/outergod/core-ops/releases/latest"><img src="https://img.shields.io/github/v/release/outergod/core-ops" alt="Latest Release"></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0--or--later-blue" alt="License: AGPL-3.0-or-later"></a>
</p>

---

## 30-second mental model

CoreOps is a convergence engine for systemd-based hosts. You declare desired
state in a Git repository — services and host overlays expressed as Quadlet
units, systemd drop-ins, and config files — and CoreOps converges the host
to match.

It treats systemd and Quadlet as the source of truth. It does not replace
them with a custom orchestrator and does not require image rebuilds for
every change. Reconciliation is declarative, idempotent, and inspectable:

* `core-ops plan` shows the diff between desired and observed state.
* `core-ops apply` makes the host match.
* `core-ops status` reports applied state with provenance back to the
  Git revision that produced it.

Each step is dry-runnable, audit-trail-producing, and re-runnable.
Re-running against a converged host is a no-op.

CoreOps is host-native: it runs on the target host (typically Fedora CoreOS),
not in a container, and operates directly on systemd state. Workloads are
container workloads (Podman/Quadlet), but the controller is not.

---

## Architecture

```mermaid
%%{init: {'flowchart': {'nodeSpacing': 50, 'rankSpacing': 70, 'padding': 20}}}%%
flowchart LR
  GIT[Git repository<br/>services/ + hosts/]
  CORE[core-ops<br/>plan / apply / explain]
  STATE[systemd + Quadlet units<br/>generated state]
  HOST[host<br/>systemd-managed services]
  AUDIT[(audit + status<br/>JSON snapshot)]
  GIT --> CORE
  CORE --> STATE
  STATE --> HOST
  CORE -.-> AUDIT
  HOST -.-> AUDIT
```

**Read left-to-right.** A Git repository (`services/` + `hosts/`)
feeds `core-ops` (`plan` / `apply` / `explain`), which generates
systemd + Quadlet units that the host runs under systemd. Audit and
status are JSON side outputs of both `core-ops` and the host — the
same architecture the diagram above shows when GitHub renders it.

---

## What using CoreOps feels like

You declare desired state in a Git repository and ask `core-ops` what
it will do before doing anything. Plan output for the canonical Immich
walkthrough on a clean host:

```text
Plan for host example @ (stateless) (first run)
───────────────────────────────────────────────
[+] Create • 10

[+] container/immich-database.container			missing
    requires
      ├─ [+] network/immich-internal.network		missing
      └─ [+] volume/immich-db-data.volume		missing
    Δ content (21 additions)
      + [Unit]
      + Description=Immich Postgres + pgvecto.rs database
      ...
...

Summary
───────
10 creates
```

After `core-ops apply` converges the host, re-running the same
invocation produces no changes — the host is already where the
declaration says it should be:

```text
Plan for host example @ (stateless)
───────────────────────────────────
[·] Unchanged • 10

[·] container/immich-database.container			unchanged
[·] container/immich-server.container			unchanged
[·] container/traefik-edge.container			unchanged
...

Summary
───────
10 unchanged
```

<p align="center">
  <a href="https://asciinema.org/a/CAST_ID">
    <img src="docs/assets/core-ops-demo.gif" alt="CoreOps terminal demo: plan, diff, explain, and apply" width="820">
  </a>
</p>

<p align="center">
  <a href="https://asciinema.org/a/CAST_ID">Watch the full terminal session on asciinema</a>
</p>

---

## Real-world examples

Five real-world homelab setups translated into the source-repository
layout. Each is runnable via stateless `--source-repo` invocation
without `core-ops init`. See `examples/<NN-slug>/README.md` for setup
intent, sources, and known limitations.

* [`examples/01-caddy-whoami`](examples/01-caddy-whoami) — Caddy reverse proxy fronting whoami (single-Container baseline).
* [`examples/02-nextcloud`](examples/02-nextcloud) — Nextcloud + Postgres + Redis + Traefik (multi-Container, intra-service network, persistent storage).
* [`examples/03-immich`](examples/03-immich) — Immich photo server with ML worker (GPU device, multi-network).
* [`examples/04-traefik-authelia`](examples/04-traefik-authelia) — Traefik + Authelia + protected backend (cross-service ForwardAuth composition).
* [`examples/05-observability`](examples/05-observability) — Prometheus + Grafana + node-exporter + cadvisor (host-scope sidecars).

Try one without committing to anything:

```sh
core-ops plan --source-repo examples/01-caddy-whoami --host example
```

No prior `core-ops init` required; nothing is written under
`/var/lib/core-ops/`. To switch into long-lived tracking mode after
copying an example to your own setup directory, run
`git init && core-ops init <path> <ref>` once.

---

## Quick start

After installing the binary:

```bash
core-ops --version
core-ops status
```

A valid installation should:

* report a build identity
* expose current system state
* produce stable, inspectable output

CoreOps is currently distributed as direct binaries for `x86_64` (`amd64`)
and `aarch64` (`arm64`).

Download the published release bundle for your target architecture. A supported
bundle includes:

- `core-ops-linux-<arch>`
- `core-ops.service`
- `core-ops.timer`
- `LICENSE`
- `CHANGELOG.md`
- `README.md`

```bash
tar -xzf core-ops-linux-<arch>.tar.gz
install -m 0755 core-ops-linux-<arch> /usr/local/bin/core-ops
install -m 0644 core-ops.service /etc/systemd/system/core-ops.service
install -m 0644 core-ops.timer /etc/systemd/system/core-ops.timer
```

No external runtime dependencies are required beyond a supported host.

For unattended host-native execution, the supported integration path uses the
published canonical `core-ops.service` and `core-ops.timer` units (also
available in `systemd/` in this repository). Initialize once, then enable the
timer:

```bash
# One-time setup: persist repository and tracking ref
core-ops init <repository-url> <ref>

# Install and enable the timer
install -m 0644 core-ops.service /etc/systemd/system/core-ops.service
install -m 0644 core-ops.timer /etc/systemd/system/core-ops.timer
systemctl daemon-reload
systemctl enable --now core-ops.timer
```

To override the quadlet directory or other defaults, use a systemd drop-in:

```bash
systemctl edit core-ops.service
```

### Supported systems

- **Supported:** Fedora CoreOS  
- **Expected to work:** other systemd-based hosts (untested)  
- **Unsupported:** non-systemd environments  

CoreOps operates directly on host-level systemd state and running CoreOps from
a container is not a supported consumption method.

---

## Why CoreOps exists

Systemd-based hosts already have a clear operating model: units define
behavior, and the system converges toward that definition. Most tooling
around them either replaces the model (Kubernetes, orchestration layers)
or ignores it (imperative configuration management).

CoreOps stays inside the model. It treats systemd and Quadlet as the source
of truth and builds a convergence workflow around them instead of replacing
them.

---

## What CoreOps is not

- Kubernetes or general container orchestration  
- A replacement for systemd  
- Generic imperative configuration management (e.g. Ansible-style)  
- A custom templating language or DSL  
- Fleet orchestration across many hosts (at this stage)  

---

## Trust and release model

CoreOps modifies host-level systemd and Quadlet artifacts in explicitly
configured locations. Operators audit behavior through plan output before
changes, apply and verification reports during changes, persisted provenance
and status after changes, and release identity and changelog continuity.
Recovery happens through explicit reconciliation, not silent mutation.

| Signal | Value |
|--------|-------|
| Published artifacts | `x86_64 raw binary`, `aarch64 raw binary`, `x86_64 tar.gz + checksums`, `aarch64 tar.gz + checksums` |
| Verification environment | `fedora-coreos-self-hosted@2026-04-fcos` |

The badges at the top of this README reflect live CI health and the latest
published release; they update automatically as the project evolves.

CoreOps defines its public guarantees through a maintained specification,
executable VM-backed verification scenarios, and a release gate. A build is
only considered distribution-ready once the release gate passes; the
verification environment is versioned to detect drift over time.

Releasable changes carry explicit SemVer intent, update the canonical version
in `Cargo.toml`, add a checked-in release fragment at `changes/<change-id>.md`,
and keep `CHANGELOG.md` current. Maintainers and CI validate this contract
through `cargo run --bin core-ops-release -- validate`. Post-merge, the
release job promotes the rendered `[Unreleased]` block to a tagged section
and publishes a GitHub Release at the merge commit; `core-ops-release promote`
owns that transition idempotently.

---

## AI authorship

CoreOps is developed with AI assistance.

AI influences how the system is produced, not how it behaves.

Behavioral guarantees come from:

* the specification
* the test corpus
* the release gate

---

## Target audience · License · Further reading

* Homelab operators working with systemd-based hosts
* Small and medium infrastructure teams
* Operators who prefer inspectable, host-native workflows

CoreOps is licensed under the GNU Affero General Public License v3 or later
(AGPLv3+). See [LICENSE](LICENSE).

* [CHANGELOG.md](CHANGELOG.md)
* [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
* [docs/development.md](docs/development.md)

Externally visible changes are tracked in [CHANGELOG.md](CHANGELOG.md) using
Keep a Changelog format.
