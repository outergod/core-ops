# Example 02 — Variant Config Root

A service whose deployment target differs from its identifier. The
service id is `traefik-dnschallenge` (descriptive, namespaceable in
the source repo) but its config payload deploys to `/etc/traefik/`,
the path the upstream binary expects. The mapping is declared in
`service.yaml` via `config-root: traefik`.

This example backs spec 016 User Story 1, acceptance scenario 2.

## Tree

```text
02-variant-config-root/
├── services/
│   └── traefik-dnschallenge/
│       ├── service.yaml             # config-root: traefik
│       ├── quadlet/
│       │   └── traefik-dnschallenge.container
│       └── config/
│           └── traefik.yaml
└── hosts/
    └── example-host/
        └── host.yaml
```

## Dispatch

| Source file | Host destination |
|---|---|
| `services/traefik-dnschallenge/quadlet/traefik-dnschallenge.container` | `/etc/containers/systemd/traefik-dnschallenge.container` |
| `services/traefik-dnschallenge/config/traefik.yaml` | `/etc/traefik/traefik.yaml` |

The config file deploys to `/etc/traefik/traefik.yaml`, NOT
`/etc/traefik-dnschallenge/traefik.yaml`. The unit filename is
unaffected — it retains the service identifier for clarity in
`systemctl list-units` output.

## What this example demonstrates

- The `service.yaml` schema: a single optional key `config-root`
  (string), kebab-case, no other keys permitted.
- Identity vs deployment target: the service identifier is free-form
  and human-meaningful; the deployment target is dictated by what the
  binary expects on disk.
- Invariant from spec 016: a service identifier never appears in a
  deployed file path unless it equals its `config-root`.

## What this example does NOT demonstrate

- The default-`config-root` rule (see `01-minimal-single-service/`).
- Drop-ins or multi-unit services (see `03-multi-unit-with-dropins/`).
- Host overlays (see `04-host-overlay/`).

## Try it

```bash
core-ops plan --source-repo . --host example-host
```

The expected plan deploys the container under
`/etc/containers/systemd/` and the config under `/etc/traefik/`.
