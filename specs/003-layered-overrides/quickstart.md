# Quickstart: Layered Overrides for Reusable Desired State

**Goal**: Reuse shared service definitions across hosts using native drop-ins and
host selection, without templating.

## Repository Layout

- Place shared services under `services/`.
- For each host, create `hosts/<host>/host.yaml` with a service list.
- Put host-specific drop-ins under `hosts/<host>/overrides/`.

Example `hosts/kadath/host.yaml`:

```yaml
host: kadath
services:
  - traefik
  - immich
```

## Host Identity Selection

- Default: OS hostname.
- Override: supply a CLI/env host override when running the controller (e.g., `CORE_OPS_HOST=ulthar` or `--host ulthar`).

## Evaluation Flow

1. Load base artifacts from `services/<service>/` for selected services.
2. Apply base drop-ins in lexicographic order.
3. Apply host overrides in lexicographic order after base drop-ins.
   For socket drop-ins, host filenames must sort after base filenames
   (e.g., `90-host.conf`).
4. Produce a concrete desired state and proceed with normal diff/plan/apply.

## Validation Rules

- Undefined services in `host.yaml` fail evaluation.
- Drop-ins targeting missing artifacts fail evaluation.
- Unsupported file types/extensions fail evaluation.
