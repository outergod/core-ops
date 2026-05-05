# 01 — Caddy + whoami

Single-Container baseline: a Caddy reverse proxy fronting a `whoami`
HTTP echo backend over a shared Quadlet network. Default config-root
(`/etc/caddy/`). Shape coverage: one service, one Quadlet `*.container`,
plus auxiliary Quadlet `*.network` and `*.volume` units.

## Pressure axis

Single-Container baseline. Validates that the spec/016 layout supports
a minimal real-world reverse-proxy + backend pattern with persistent
state (Caddy automatically issues and stages TLS certificates into the
`caddy-data` volume).

## Sources

These references shaped the Quadlet equivalents. Upstream YAML/compose
blocks were not copied verbatim (research.md D5 license hygiene).

- Caddy quick-start: <https://caddyserver.com/docs/quick-starts/reverse-proxy>
- Caddy Docker official image: <https://hub.docker.com/_/caddy>
- traefik/whoami container README: <https://hub.docker.com/r/traefik/whoami>

## Service-by-service intent

| Service | Image | Purpose | Notes |
|---------|-------|---------|-------|
| `caddy` | `docker.io/library/caddy:2` | TLS terminator + reverse proxy | Mounts `/etc/caddy/Caddyfile` (default config-root); state in `caddy-data` volume |
| `whoami` | `docker.io/traefik/whoami` | HTTP echo backend | Joined to the same `caddy` network |

## Try it

> CLI output below is illustrative and not snapshot-tested.

```sh
core-ops plan --source-repo examples/01-caddy-whoami --host example
```

Expected: exit 0; plan lists the Caddy container, the whoami container,
the shared network, and the two Caddy volumes. No prior `core-ops init`
required; nothing written under `/var/lib/core-ops/`.

## Known limitations

None encountered during translation — this example is the spec/016
layout's narrowest shape and exercises no friction beyond the parser
contract.

## Scaffold for your own setup

```sh
cp -r examples/01-caddy-whoami ~/my-caddy
# Edit hosts/example/host.yaml → rename `example` to your host id.
# Edit services/caddy/config/Caddyfile → set your real domain + backend.
core-ops plan --source-repo ~/my-caddy --host <your-host>
```

Once happy, `git init && core-ops init ~/my-caddy main` to switch into
long-lived tracking mode.
