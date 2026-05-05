# 03 — Immich photo server with ML worker

Immich photo/video library: server + Postgres (pgvecto.rs) + Redis +
ML inference worker + Traefik edge proxy. Exercises GPU device
passthrough (host overlay drop-in on `immich-ml`) and multi-network
membership (`immich-server` joins both an internal and a public
network so the edge proxy can reach it).

## Pressure axis

GPU device passthrough, multi-network membership, ML worker. Validates
that the spec/016 layout supports a real workload where one service
joins multiple Quadlet networks and one service receives a host-side
device drop-in.

## Sources

These references shaped the Quadlet equivalents. Upstream YAML/compose
blocks were not copied verbatim (research.md D5 license hygiene).

- Immich docker-compose example:
  <https://github.com/immich-app/immich/blob/main/docker/docker-compose.yml>
- Immich Postgres image (pgvecto.rs flavor):
  <https://github.com/immich-app/base-images>
- Podman CDI for NVIDIA GPUs:
  <https://github.com/containers/podman/blob/main/docs/source/markdown/podman-run.1.md.in>
- Intel/AMD VAAPI on `/dev/dri`:
  <https://docs.kernel.org/userspace-api/dma-buf-alloc-exchange.html>

## Service-by-service intent

| Service | Image | Purpose | Notes |
|---------|-------|---------|-------|
| `immich-server` | `ghcr.io/immich-app/immich-server:release` | Headlining app server | Joins both `immich-internal` and `immich-public` networks |
| `immich-database` | `ghcr.io/immich-app/postgres:16` | Postgres + pgvecto.rs | Internal network only |
| `immich-redis` | `docker.io/library/redis:7-alpine` | In-memory cache | Internal network only |
| `immich-ml` | `ghcr.io/immich-app/immich-machine-learning:release` | ML inference worker | Receives `AddDevice=/dev/dri` via host overlay drop-in |
| `traefik-edge` | `docker.io/library/traefik:v3.1` | Edge reverse proxy | Public network only; reaches `immich-server` via shared network |

## Try it

> CLI output below is illustrative and not snapshot-tested.

```sh
core-ops plan --source-repo examples/03-immich --host example
```

Expected: exit 0; plan lists 5 containers, 2 networks, 3 volumes, and
the host-side `immich-ml.container.d/20-gpu.conf` drop-in adding GPU
device passthrough.

## Known limitations

- **GPU shape is host-specific**: the example ships an Intel/AMD VAAPI
  drop-in (`AddDevice=/dev/dri:/dev/dri`). For NVIDIA, the operator
  must rewrite to CDI (`AddDevice=nvidia.com/gpu=all` or
  `PodmanArgs=--device nvidia.com/gpu=all`) and ensure the
  nvidia-container-toolkit + CDI spec is installed on the host.
  Synthesis table classification: `B` — workaround documented here;
  no layout change is required.
- **Secrets are referenced, not committed**: `immich-db-password` is a
  Podman secret the operator must create on the host
  (`podman secret create immich-db-password ...`) before applying.
- **Library mount is in-host**: the example uses a Quadlet `*.volume`
  for uploads (`immich-upload`). Real homelab deployments often back
  this with NFS. NFS mount declarations are out-of-scope for this
  example; see the synthesis table for tracking.

## Scaffold for your own setup

```sh
cp -r examples/03-immich ~/my-immich
# Edit hosts/example/host.yaml → rename `example` to your host id.
# If on NVIDIA, edit the GPU drop-in; if on Intel/AMD VAAPI, leave it.
# Create Podman secret on the target host before applying.
core-ops plan --source-repo ~/my-immich --host <your-host>
```
