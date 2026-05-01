# Quickstart: Authoring a CoreOps Source Repository

**Feature**: 016-source-repository-layout
**Audience**: Operators, contributors, agents authoring a source repository.

This walkthrough takes you from an empty directory to a working source repository with one minimal service, one variant service, and one host overlay. It assumes `core-ops` v2.0.0 (the binary that consumes the formalized layout).

---

## 1. Create the skeleton

```bash
mkdir -p my-fleet/services my-fleet/hosts
cd my-fleet
git init -q
```

A source repository is just two top-level directories: `services/` and `hosts/`.

## 2. Add a minimal service

Most services have an identifier that equals their `/etc/` destination. No `service.yaml` is needed.

```bash
mkdir -p services/whoami/quadlet services/whoami/config

cat > services/whoami/quadlet/whoami.container <<'EOF'
[Unit]
Description=whoami HTTP echo
[Container]
Image=docker.io/traefik/whoami:latest
PublishPort=8080:80
[Install]
WantedBy=multi-user.target
EOF

cat > services/whoami/config/whoami.toml <<'EOF'
# This file deploys to /etc/whoami/whoami.toml
log-level = "info"
EOF
```

That's it. `services/whoami/config/whoami.toml` will land at `/etc/whoami/whoami.toml` because the service identifier `whoami` is also its config root.

## 3. Add a variant service (config root differs from identifier)

When you want to run a configured Traefik but call the service `traefik-dnschallenge`, declare the config root explicitly:

```bash
mkdir -p services/traefik-dnschallenge/{quadlet,systemd,config}

cat > services/traefik-dnschallenge/service.yaml <<'EOF'
config-root: traefik
EOF

cat > services/traefik-dnschallenge/quadlet/traefik.container <<'EOF'
[Unit]
Description=Traefik (DNS challenge)
[Container]
Image=docker.io/library/traefik:latest
[Install]
WantedBy=multi-user.target
EOF

cat > services/traefik-dnschallenge/systemd/traefik.socket <<'EOF'
[Unit]
Description=Traefik HTTP socket
[Socket]
ListenStream=80
[Install]
WantedBy=sockets.target
EOF

cat > services/traefik-dnschallenge/config/traefik.toml <<'EOF'
# Deploys to /etc/traefik/traefik.toml because of config-root above
[entryPoints]
EOF
```

Note the split: the `*.container` Quadlet input lives under `quadlet/`, the `*.socket` native systemd unit lives under `systemd/`. Both belong to the same logical service.

## 4. Declare a host

```bash
mkdir -p hosts/laptop

cat > hosts/laptop/host.yaml <<'EOF'
host: laptop
services:
  - whoami
  - traefik-dnschallenge
EOF
```

The `host.yaml` is required. Its `host` field MUST equal the directory name. The `services` list determines which services apply to this host, in order.

## 5. Add a host overlay (drop-in + config replacement)

A host can contribute drop-ins and replace config files for any service it selects. The overlay tree mirrors the service tree directly under `hosts/<host-id>/<svc-id>/`:

```bash
mkdir -p hosts/laptop/traefik-dnschallenge/quadlet/traefik.container.d
mkdir -p hosts/laptop/traefik-dnschallenge/config

cat > hosts/laptop/traefik-dnschallenge/quadlet/traefik.container.d/10-dashboard.conf <<'EOF'
[Container]
Label=traefik.http.routers.dashboard.rule=Host(`laptop.local`)
EOF

cat > hosts/laptop/traefik-dnschallenge/config/traefik.toml <<'EOF'
# Whole-file replacement of /etc/traefik/traefik.toml on the laptop host.
[entryPoints]
  [entryPoints.web]
    address = ":80"
EOF
```

A host overlay can NOT introduce a new base unit (no `*.container` directly under `quadlet/`). Only drop-ins and `config/` replacements.

## 6. Plan and apply

```bash
core-ops init .                          # initialize the controller against this source repo
core-ops plan                            # show what will change on the host
core-ops apply                           # converge the host
```

Plan output reports source-repo path, host identifier, the resolved service set, drop-in merge order, and the destination path for every action.

## 7. Install the agent skill

If you (or an agent collaborating with you) want authoring guidance available in this repository:

```bash
core-ops skill install                   # writes ./.agents/skills/core-ops-source-repo/
core-ops skill install --global          # writes ~/.agents/skills/core-ops-source-repo/
core-ops skill install --print > skill.txt  # for inspection or piping into other tooling
```

The bundle includes a `SKILL.md` describing the canonical layout, the `service.yaml` schema, the payload-kind dispatch, host-overlay semantics, drop-in conventions, validation rules, and a worked authoring walk-through. Agents that read it can author conformant services without reading source code.

The path standard is `.agents/skills/<skill-name>/` (agentskills.io). The binary does not write to vendor-specific paths.

---

## Migrating from a legacy layout

If you have an older source repository under the pre-v2 layout (`services/<svc>/quadlet-overrides/`, `hosts/<h>/overrides/`, `services/<svc>/config/etc/<svc>/`):

```bash
scripts/migrate-legacy-source-repo.sh <path-to-your-source-repo>
```

The script is idempotent and operates by file moves only — no semantic re-interpretation. After running, `core-ops plan` produces the same set of host destination paths as before; only source paths in the repository change.

---

## Reference example repositories

For richer worked examples, see:

- `specs/016-source-repository-layout/examples/01-minimal-single-service/`
- `specs/016-source-repository-layout/examples/02-variant-config-root/`
- `specs/016-source-repository-layout/examples/03-multi-unit-with-dropins/`
- `specs/016-source-repository-layout/examples/04-host-overlay/`

Each example is a self-contained source repository directory tree that loads cleanly under v2.0.0.
