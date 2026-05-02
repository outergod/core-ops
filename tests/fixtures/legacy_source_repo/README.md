# Legacy source-repo migration fixture

This directory is a minimal source repository in the **legacy** (pre-spec-016) shape, exercising every transformation in `specs/016-source-repository-layout/research.md` D10:

| Legacy artifact | Migration outcome |
|---|---|
| `services/traefik/quadlet/traefik.container` | unchanged |
| `services/traefik/quadlet/traefik.socket` | reassigned to `services/traefik/systemd/` |
| `services/traefik/quadlet-overrides/traefik.container.d/10-defaults.conf` | merged into `services/traefik/quadlet/traefik.container.d/` |
| `services/traefik/config/etc/traefik/traefik.toml` | flattened to `services/traefik/config/traefik.toml` (config-root matches svc-id, no service.yaml needed) |
| `services/traefik-dnschallenge/config/etc/traefik/cert.toml` | flattened to `services/traefik-dnschallenge/config/cert.toml` PLUS a generated `service.yaml` declaring `config-root: traefik` (variant case) |
| `hosts/example-host/overrides/quadlet/traefik.container.d/20-host.conf` | per-service: `hosts/example-host/traefik/quadlet/traefik.container.d/20-host.conf` |
| `hosts/example-host/overrides/config/etc/traefik/traefik.toml` | per-service: `hosts/example-host/traefik/config/traefik.toml` |

The fixture is **read-only**: `tests/integration/test_migrate_legacy.rs` copies it to a temp directory and runs `scripts/migrate-legacy-source-repo.sh` against the copy.

`expected-destinations.txt` records the post-migration `core-ops plan` destination set (sorted, one per line). This is the contract: pre-migration plan parity (SC-003) was the design goal, but since the legacy parser is gone, the test asserts equality between the migrated tree's destinations and this hand-recorded list.
