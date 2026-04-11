# CoreOps

<p align="center">
  <img src="docs/core-ops.svg" alt="CoreOps logo" width="160">
</p>

<p align="center">
  <strong>Host-native convergence for systemd-based systems</strong>
</p>

---

## What is CoreOps?

CoreOps is a convergence engine for systemd-based hosts.

It takes a declared system state and makes the host match it —  
without hiding systemd, without introducing a new orchestration layer,  
and without requiring image rebuilds for every change.

If you are managing Fedora CoreOS or similar systems and find yourself
choosing between:

- rebuilding images for every iteration, or  
- drifting into imperative host configuration  

CoreOps exists in that gap.

---

## Why CoreOps Exists

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

## What CoreOps Does

- Converges host-managed workloads declaratively  
- Works with systemd-native and Quadlet-native workflows  
- Produces inspectable plans, reports, and machine-readable output  
- Preserves provenance and reconciliation state for later audit  
- Executes VM-backed verification scenarios against real systems  

---

## What CoreOps Is Not

- Kubernetes or general container orchestration  
- A replacement for systemd  
- Generic imperative configuration management (e.g. Ansible-style)  
- A custom templating language or DSL  
- Fleet orchestration across many hosts (at this stage)  

---

## Supported Systems

- **Supported:** Fedora CoreOS  
- **Expected to work:** other systemd-based hosts (untested)  
- **Unsupported:** non-systemd environments  

CoreOps operates directly on host-level systemd state and running CoreOps from
a container is not a supported consumption method.

---

## Credibility

[![CI](https://github.com/outergod/core-ops/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/outergod/core-ops/actions/workflows/ci.yml)
[![E2E Gate](https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml/badge.svg)](https://github.com/outergod/core-ops/actions/workflows/e2e-gate.yml)
[![Latest Release](https://img.shields.io/github/v/release/outergod/core-ops)](https://github.com/outergod/core-ops/releases/latest)

| Signal | Value |
|--------|-------|
| Published artifacts | `x86_64 raw binary`, `aarch64 raw binary`, `x86_64 tar.gz + checksums`, `aarch64 tar.gz + checksums` |
| Verification environment | `fedora-coreos-self-hosted@2026-04-fcos` |

This section is the stable credibility surface for outside evaluators. The badges
above reflect live CI health and the latest published release version and update
automatically as the project evolves.

---

## First Interaction

After installing the binary:

```bash
core-ops --version
core-ops status
``` 

A valid installation should:

* report a build identity
* expose current system state
* produce stable, inspectable output

---

## Installation (Current Phase)

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
published canonical `core-ops.service` and `core-ops.timer` units. After
placing those unit files, configure them through a systemd drop-in and enable
the timer:

```bash
systemctl edit core-ops.service
systemctl daemon-reload
systemctl enable core-ops.service
systemctl enable --now core-ops.timer
```

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

---

## Target Audience

* Homelab operators working with systemd-based hosts
* Small and medium infrastructure teams
* Operators who prefer inspectable, host-native workflows

---

## AI Authorship

CoreOps is developed with AI assistance.

AI influences how the system is produced, not how it behaves.

Behavioral guarantees come from:

* the specification
* the test corpus
* the release gate

---

## License

CoreOps is licensed under the GNU Affero General Public License v3 or later
(AGPLv3+).

See [LICENSE](LICENSE).

---

## Further Reading

* [CHANGELOG.md](CHANGELOG.md)
* [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
* [docs/development.md](docs/development.md)

Externally visible changes are tracked in [CHANGELOG.md](CHANGELOG.md) using
Keep a Changelog format.
