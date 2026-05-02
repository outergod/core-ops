# Example 01 — Minimal Single Service

The simplest conformant CoreOps source repository under layout version `1`:
one service, one Quadlet container, one config file, one host. No
`service.yaml` — the service's `config-root` defaults to its identifier.

This example backs spec 016 User Story 1, acceptance scenario 1.

## Tree

```text
01-minimal-single-service/
├── services/
│   └── whoami/
│       ├── quadlet/
│       │   └── whoami.container
│       └── config/
│           └── whoami.toml
└── hosts/
    └── example-host/
        └── host.yaml
```

No `service.yaml` is present; the service's `config-root` is therefore
`whoami` (defaulted from the directory name).

## Dispatch (what `core-ops plan` produces against `example-host`)

| Source file | Host destination |
|---|---|
| `services/whoami/quadlet/whoami.container` | `/etc/containers/systemd/whoami.container` |
| `services/whoami/config/whoami.toml` | `/etc/whoami/whoami.toml` |

## What this example demonstrates

- The default-`config-root` rule: a service with no `service.yaml`
  deploys its `config/` payload to `/etc/<svc-id>/`.
- The minimum viable host: `host.yaml` lists exactly the services it
  applies, in order. No host overlay is required if the host has nothing
  to override.
- Quadlet payload dispatch: a `*.container` under a service's `quadlet/`
  directory deploys to `/etc/containers/systemd/`, the standard Quadlet
  search path.

## What this example does NOT demonstrate

- Variant `config-root` (see `02-variant-config-root/`).
- Multi-unit services or drop-ins (see `03-multi-unit-with-dropins/`).
- Host overlays (see `04-host-overlay/`).

## Try it

From this directory:

```bash
core-ops plan --source-repo . --host example-host
```

The expected plan applies the two destinations above and nothing else.
