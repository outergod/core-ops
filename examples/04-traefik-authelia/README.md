# 04 — Traefik + Authelia + protected backend

Cross-service ForwardAuth composition: Traefik fronts a `whoami` backend
that is protected by Authelia via Traefik's `forwardAuth` middleware.
The middleware wiring lives in Traefik's static config; the host-side
drop-in on `whoami` selects which router/middleware to apply, so the
auth policy is layered on at host time rather than baked into the
service's base unit.

## Pressure axis

Cross-service ForwardAuth composition. Validates that the spec/016
layout supports a real-world auth pattern where one service (Authelia)
provides a side-effect to another service (whoami) via a third
service's labels (Traefik).

## Sources

These references shaped the Quadlet equivalents. Upstream YAML/compose
blocks were not copied verbatim (research.md D5 license hygiene).

- Authelia Traefik integration:
  <https://www.authelia.com/integration/proxies/traefik/>
- Traefik forwardAuth middleware:
  <https://doc.traefik.io/traefik/middlewares/http/forwardauth/>
- traefik/whoami container README:
  <https://hub.docker.com/r/traefik/whoami>

## Service-by-service intent

| Service | Image | Purpose | Notes |
|---------|-------|---------|-------|
| `traefik` | `docker.io/library/traefik:v3.1` | Edge reverse proxy | Static config declares the `authelia` ForwardAuth middleware |
| `authelia` | `docker.io/authelia/authelia:4` | Identity provider + ForwardAuth target | Default config-root; reachable on `auth.network` at `http://authelia:9091` |
| `whoami` | `docker.io/traefik/whoami:latest` | Generic protected backend | Base unit is plain; host overlay adds Traefik labels for the auth router |

## Try it

> CLI output below is illustrative and not snapshot-tested.

```sh
core-ops plan --source-repo examples/04-traefik-authelia --host example
```

Expected: exit 0; plan lists 3 containers, 1 network, 2 config files
(`/etc/traefik/traefik.yaml` + `/etc/authelia/configuration.yml`), and
the host-side `whoami.container.d/10-forwardauth.conf` drop-in adding
Traefik labels for the protected router.

## Known limitations

- **Users database stub**: Authelia expects a `users_database.yml` next
  to its main config. The example does not commit one (FR-009 — no
  fake or real credentials). Operators must populate
  `/etc/authelia/users_database.yml` on the host before applying.
  Synthesis table classification: `B` — workaround documented here.
- **No TLS certificate provider**: `entryPoints.websecure` is declared
  but no certResolver is wired up. Real deployments need ACME (DNS-01
  or HTTP-01) configured against the operator's domain. Out of scope
  for this example.
- **Authelia secrets are external**: JWT secret, session secret, and
  storage encryption key all need to be sourced from a secrets backend
  (`AUTHELIA_*_FILE` env vars are the standard pattern). Not committed
  here.

## Scaffold for your own setup

```sh
cp -r examples/04-traefik-authelia ~/my-auth
# Edit hosts/example/host.yaml → rename `example` to your host id.
# Edit services/authelia/config/configuration.yml → set domain, ACL.
# Populate /etc/authelia/users_database.yml on the host before applying.
core-ops plan --source-repo ~/my-auth --host <your-host>
```
