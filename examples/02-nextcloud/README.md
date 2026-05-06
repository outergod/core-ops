# 02 — Nextcloud (community multi-container)

Multi-Container homelab Nextcloud stack: Nextcloud + Postgres + Redis +
Traefik edge proxy. Intra-service Quadlet network, persistent storage
volumes, host-side TLS port drop-in. The `traefik-edge` service id
diverges from its `config-root: traefik`, exercising the
`service.yaml` redirection path.

## Pressure axis

Multi-Container, intra-service network, persistent storage. Validates
that the spec/016 layout supports a real-world four-container stack
where each container is its own service directory and the headlining
service (`nextcloud`) depends on its peers via Quadlet `Requires=`.

## Sources

These references shaped the Quadlet equivalents. Upstream YAML/compose
blocks were not copied verbatim (research.md D5 license hygiene).

- Nextcloud official Docker image: <https://hub.docker.com/_/nextcloud>
- Nextcloud community Docker examples (NOT the All-In-One container,
  which manages its own sub-containers via the Docker socket and is
  incompatible with external orchestration):
  <https://github.com/nextcloud/docker/tree/master/.examples/docker-compose>
- Postgres official image: <https://hub.docker.com/_/postgres>
- Redis official image: <https://hub.docker.com/_/redis>
- Traefik v3 docs: <https://doc.traefik.io/traefik/>

## Service-by-service intent

| Service | Image | Purpose | Notes |
|---------|-------|---------|-------|
| `nextcloud` | `docker.io/library/nextcloud:30` | Headlining Nextcloud app server | Mounts `nextcloud-data` volume; declares `Requires=` on db + redis |
| `nextcloud-db` | `docker.io/library/postgres:16` | Postgres backing store | Persistent `nextcloud-db-data` volume; password sourced via Podman secret |
| `nextcloud-redis` | `docker.io/library/redis:7-alpine` | In-memory cache | Save disabled (cache only) |
| `traefik-edge` | `docker.io/library/traefik:v3.1` | Edge reverse proxy | Service id `traefik-edge`, `config-root: traefik` (config-root divergence) |

## Try it

> CLI output below is illustrative and not snapshot-tested.

```sh
core-ops plan --source-repo examples/02-nextcloud --host example
```

Expected: exit 0; plan lists 4 containers, 1 network, 2 volumes, 1
config file (`/etc/traefik/traefik.yaml` — note the `traefik-edge` →
`traefik` config-root rewrite), and the host-side `traefik-edge.container.d/10-tls.conf`
drop-in adding the TLS port.

## Known limitations

- **Secrets are referenced, not committed**: the example declares a
  Podman secret `nextcloud-db-password` but does not provide its
  contents. Operators must `podman secret create nextcloud-db-password
  /path/to/secret` on the host before applying. Secret bootstrap
  belongs to the host, not the source-repo (FR-009: no real values).
- **Trusted domain placeholder**: `NEXTCLOUD_TRUSTED_DOMAINS` is set to
  `cloud.example.com` (RFC 2606). Replace with the operator's real
  domain in their own scaffold copy before applying.
- **Initial Nextcloud setup is interactive**: the first `apply`
  installs files; the operator still needs to complete the install
  wizard at `http://<host>/` to create the admin account. This is a
  Nextcloud product behavior, not a layout limitation. (Synthesis
  table classification: `B` — workaround documented here.)

## Scaffold for your own setup

```sh
cp -r examples/02-nextcloud ~/my-nextcloud
# Edit hosts/example/host.yaml → rename `example` to your host id.
# Edit services/traefik-edge/config/traefik.yaml → set your domain.
# `podman secret create nextcloud-db-password ...` on the target host.
core-ops plan --source-repo ~/my-nextcloud --host <your-host>
```
