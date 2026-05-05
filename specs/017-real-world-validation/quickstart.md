# Quickstart: Real-World Examples (operator walkthrough)

**Phase**: 1 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

This is the operator-facing walkthrough that the post-implementation `examples/<NN-slug>/README.md` files will mirror, and which the root `README.md` "Real-World Examples" section will link to. It is a verification artifact for `/speckit.plan`: if these steps cannot be followed end-to-end against the implemented feature, the implementation is not complete.

---

## Prerequisites

- `core-ops` v2.2.0 or later (this slice). Stateless `--source-repo` requires the new flag.
- A clone of this repository, OR a downloaded `examples/<NN-slug>/` subtree.
- `git` available on `$PATH` (used for git-aware provenance detection; non-git directories also work).
- No prior `core-ops init` required.

---

## Five-minute walkthrough

### Step 1 — Browse the real-world examples

```sh
git clone https://github.com/outergod/core-ops.git
cd core-ops
ls examples/
```

You see five subdirectories, each documenting one widely-deployed homelab pattern:

| Slug | Setup | Pressure axis |
|------|-------|---------------|
| `01-caddy-whoami` | Caddy + whoami | Single-Container baseline; default config-root |
| `02-nextcloud` | Nextcloud + Postgres + Redis + Traefik | Multi-Container, intra-service network, persistent storage |
| `03-immich` | Immich + Postgres + Redis + ML + Traefik | GPU device, multi-network, ML worker |
| `04-traefik-authelia` | Traefik + Authelia + protected backend | Cross-service ForwardAuth composition |
| `05-observability` | Prometheus + Grafana + node-exporter + cadvisor | Host-scope sidecars, scrape-config templating |

Each example's `README.md` cites its public upstream design sources, lists the services involved, and documents any known limitations encountered during translation.

### Step 2 — Plan an example without committing to it

```sh
core-ops plan --source-repo examples/01-caddy-whoami --host example
```

This invocation:
- Reads desired state from the directory tree under `examples/01-caddy-whoami/`.
- Selects the host overlay under `hosts/example/`.
- Computes the reconciliation plan against the current host state.
- Writes nothing to `/var/lib/core-ops/`.
- Does not require — and does not consult — any prior `core-ops init`.

Expected output: a plan listing the units that would be installed, started, or modified. Exit code 0.

### Step 3 — Inspect a single object

```sh
core-ops explain --source-repo examples/01-caddy-whoami --host example caddy.container
```

Produces an authoritative explanation of how the reconciliation model interprets the `caddy.container` Quadlet inside the example. Read-only; writes nothing.

### Step 4 — Switch to a different example without `--force`

```sh
core-ops plan --source-repo examples/05-observability --host example
```

This succeeds with no teardown step. Stateless invocations against a different example are independent and do not conflict.

### Step 5 — (Optional) Apply against a real host

> ⚠ **`apply` mutates host state.** Read the plan from Step 2 before running.

```sh
core-ops apply --source-repo examples/01-caddy-whoami --host example
```

If the operator has already run `core-ops init <git-url> <ref>` against a different repository, that init'd state is left untouched by this stateless apply. Audit records and the status snapshot record the path-based provenance:

```sh
core-ops status
```

Provenance shows:
- `desired_state.repository` = `<absolute-path-to-examples/01-caddy-whoami>`.
- `desired_state.requested_ref` = the git commit SHA if the path is a clean git checkout, or `(stateless+dirty)` for a working tree with uncommitted changes, or `(stateless)` for a non-git directory.

### Step 6 — Author your own setup using an example as a scaffold

Stateless mode is the inner-loop authoring substrate:

```sh
cp -r examples/02-nextcloud ~/my-nextcloud
# Edit ~/my-nextcloud/hosts/<your-host>/host.yaml and service configs
core-ops plan --source-repo ~/my-nextcloud --host <your-host>
```

Iterate without `git init`, without `core-ops init`, without `--force`. When ready for long-lived tracking:

```sh
cd ~/my-nextcloud
git init && git add . && git commit -m "initial homelab config"
core-ops init ~/my-nextcloud main
core-ops plan      # now sources from persisted init'd state
```

### Step 7 — Reading the synthesis table

If the example's README mentions a "known limitation" you also encounter, look up the friction in `specs/017-real-world-validation/spec.md` — the synthesis table classifies every encountered friction as **A** (amend-now, escalated to a follow-up spec), **B** (workaround-with-doc, with the workaround inlined in the example's README), or **C** (defer-to-spec-018, tracked in `docs/follow-ups.md`).

---

## Acceptance check (operator self-verification)

- [ ] `core-ops plan --source-repo examples/01-caddy-whoami --host example` exits 0 and emits a non-empty plan, with no prior `core-ops init` and no writes to `/var/lib/core-ops/`. (SC-001, SC-008)
- [ ] Switching from one example to another via re-invocation does not require `--force`. (US1 AC2)
- [ ] On a host with a prior `core-ops init`, running stateless `apply --source-repo` does not mutate the persisted `desired_state.repository` / `desired_state.requested_ref` of the init'd configuration. (SC-009)
- [ ] `core-ops status` after a stateless apply shows path-based provenance (absolute path under `desired_state.repository`, SHA or sentinel under `desired_state.requested_ref`).
- [ ] All five examples parse cleanly via `cargo test` per-example integration tests. (SC-003)
- [ ] Each example's `README.md` cites at least one public upstream source URL. (SC-006)
- [ ] No file under `examples/` mentions any operator-private value (`not.one`, `ulthar`, `192.168.1.2`, GCloud DNS markers). (SC-005)
