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

<!-- T004 reorder: §4 Architecture and §5 Walkthrough go here (filled by T009). -->

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

Systemd-based hosts already have a clear operating model:

- units define behavior  
- the system converges toward that definition  

But most tooling around them does one of two things:

- replaces the model (Kubernetes, orchestration layers)  
- ignores it (imperative configuration management)  

CoreOps stays inside that model.

It treats systemd and Quadlet as the source of truth and builds a
convergence workflow around them instead of replacing them.

---

## What CoreOps is not

- Kubernetes or general container orchestration  
- A replacement for systemd  
- Generic imperative configuration management (e.g. Ansible-style)  
- A custom templating language or DSL  
- Fleet orchestration across many hosts (at this stage)  

---

<!-- T005 dissolved the "## Credibility" heading; badges promoted to top
of file. The signal table and blurb stay here for T008 to fold into
"## Trust and release model" along with Minimal Trust Story and the
Release & Verification Model. -->

| Signal | Value |
|--------|-------|
| Published artifacts | `x86_64 raw binary`, `aarch64 raw binary`, `x86_64 tar.gz + checksums`, `aarch64 tar.gz + checksums` |
| Verification environment | `fedora-coreos-self-hosted@2026-04-fcos` |

The badges at the top of this README reflect live CI health and the latest
published release version, and update automatically as the project evolves.

---

## Minimal Trust Story

CoreOps modifies host-level systemd and Quadlet artifacts in explicitly
configured locations.

Operators can audit behavior through:

* plan output before changes
* apply and verification reports during changes
* persisted provenance and status after changes
* release identity and changelog continuity

Recovery is expected to happen through explicit reconciliation and
documented retry/rollback behavior — not silent mutation.

---

## Release & Verification Model

CoreOps defines its public guarantees through:

* a maintained specification
* executable verification scenarios
* a release gate

A build is only considered distribution-ready once the release gate passes.

The verification environment is versioned to detect drift over time.

Releasable changes are expected to carry explicit SemVer intent, update the
canonical version in `Cargo.toml`, update a checked-in release fragment at
`changes/<change-id>.md`, and keep `CHANGELOG.md` current.

Maintainers and CI validate this contract through the dedicated helper binary:

```bash
cargo run --bin core-ops-release -- validate
```

Once a feature PR lands on `master`, the post-merge release job promotes the
rendered `[Unreleased]` block to a new `[<version>] - <date>` section, removes
the consumed fragments under `changes/`, and publishes the GitHub Release at
the merge commit (which also creates the git tag). Maintainers do not edit
`CHANGELOG.md` after the `[Unreleased]` block — `core-ops-release promote`
owns that transition and is idempotent on re-run.

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
