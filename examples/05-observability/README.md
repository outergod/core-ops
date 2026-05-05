# 05 — Observability stack

Prometheus + Grafana + node-exporter + cadvisor with host-scope
sidecars (`/proc`, `/sys`, `/`-rootfs bind mounts on the metric
exporters). Demonstrates the "scrape-config templating" friction (see
known limitations).

## Pressure axis

Host-scope sidecars + scrape-config templating. Validates that the
spec/016 layout supports privileged containers with bind mounts that
escape the container filesystem (`Volume=/proc:/host/proc:ro,rslave`
etc.) and surfaces the per-host scrape-config templating gap.

## Sources

These references shaped the Quadlet equivalents. Upstream YAML/compose
blocks were not copied verbatim (research.md D5 license hygiene).

- Prometheus docker-compose example:
  <https://github.com/prometheus/prometheus/blob/main/docker-compose.yml>
- node_exporter recommended bind mounts:
  <https://github.com/prometheus/node_exporter#docker>
- cadvisor recommended bind mounts:
  <https://github.com/google/cadvisor#quick-start-running-cadvisor-in-a-docker-container>
- Grafana provisioning docs:
  <https://grafana.com/docs/grafana/latest/administration/provisioning/>

## Service-by-service intent

| Service | Image | Purpose | Notes |
|---------|-------|---------|-------|
| `prometheus` | `docker.io/prom/prometheus:v2` | Metrics scraper | TSDB on `prometheus-data` volume; static targets in `prometheus.yml` |
| `grafana` | `docker.io/grafana/grafana:11` | Dashboards | Persistent state on `grafana-data` volume |
| `node-exporter` | `quay.io/prometheus/node-exporter:latest` | Host metrics | Bind-mounts `/proc`, `/sys`, `/` (read-only, rslave) |
| `cadvisor` | `gcr.io/cadvisor/cadvisor:v0.49.1` | Container metrics | Bind-mounts `/`, `/sys`, `/var/run`, `/var/lib/containers`; runs `--privileged` |

## Try it

> CLI output below is illustrative and not snapshot-tested.

```sh
core-ops plan --source-repo examples/05-observability --host example
```

Expected: exit 0; plan lists 4 containers, 1 network, 2 volumes,
2 config files (`/etc/prometheus/prometheus.yml`,
`/etc/grafana/grafana.ini`). The host overlay replaces
`prometheus.yml` with a host-tailored target list.

## Known limitations

- **Scrape-config templating gap**: Prometheus's `prometheus.yml`
  needs the list of scrape targets baked into a static file. The
  spec/016 layout has no templating layer that can compute "for each
  host, list its scrape targets" automatically — every host needs to
  ship its own `prometheus.yml` whole-file replacement under
  `hosts/<host>/prometheus/config/prometheus.yml`. This example
  demonstrates the workaround. Synthesis table classification: `B` —
  workaround documented; no layout change required for this slice.
  Future work could escalate to a templating-layer spec if multiple
  workloads need this.
- **cadvisor requires `--privileged`**: cadvisor reads cgroups via
  `/sys/fs/cgroup` and needs broader capabilities than the default
  rootless Podman profile permits. The example uses `PodmanArgs=--privileged`
  which is the documented upstream pattern. Operators on hardened
  hosts may need to substitute fine-grained capabilities.

## Scaffold for your own setup

```sh
cp -r examples/05-observability ~/my-observability
# Edit hosts/example/host.yaml → rename `example` to your host id.
# Edit hosts/<your-host>/prometheus/config/prometheus.yml → list your
# host-specific scrape targets.
core-ops plan --source-repo ~/my-observability --host <your-host>
```
