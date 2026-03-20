# Repository Layout Contract

## Purpose
Define the repository structure and host selection inputs used for layered overrides.

## Required Structure

- `services/<service>/` contains base artifacts and base drop-ins.
- `hosts/<host>/host.yaml` declares host identity and explicit service selection.
- `hosts/<host>/overrides/` contains host-specific drop-ins layered after base drop-ins.

## Host Declaration Schema

`hosts/<host>/host.yaml`:

```yaml
host: <host-id>
services:
  - <service-name>
  - <service-name>
```

### Rules

- `host` must match the `<host>` directory name.
- `services` must reference existing `services/<service>/` directories.
- No groups/roles are supported; selection is an explicit list.

## Drop-in Rules

- Quadlet drop-ins: `artifact.container.d/*.conf`, `artifact.volume.d/*.conf`.
- Systemd socket drop-ins: `artifact.socket.d/*.conf`.
- Drop-ins are applied in lexicographic order by filename.
- Host overrides are applied after base drop-ins.

## Validation Failures

- Undefined service selection → evaluation fails.
- Drop-in targets nonexistent artifact → evaluation fails.
- Unsupported file types/extensions → evaluation fails.
