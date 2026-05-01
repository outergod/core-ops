# Example 03 — Multi-Unit Service with Drop-Ins

A service that combines a Quadlet container with a native systemd
socket, with a base drop-in on each. Demonstrates how a single
service spans payload-kind directories and how drop-ins refine the
base units in lex-sorted order.

This example backs spec 016 FR-003, FR-004, FR-014 (drop-in lex
order), and the multi-unit class of FR-023.

## Tree

```text
03-multi-unit-with-dropins/
├── services/
│   └── webhook-receiver/
│       ├── quadlet/
│       │   ├── webhook-receiver.container
│       │   └── webhook-receiver.container.d/
│       │       └── 10-resources.conf
│       └── systemd/
│           ├── webhook-receiver.socket
│           └── webhook-receiver.socket.d/
│               └── 10-hardening.conf
└── hosts/
    └── example-host/
        └── host.yaml
```

No `service.yaml`; `config-root` defaults to `webhook-receiver`.
The example carries no `config/` files because the unit-and-drop-in
shape is the point.

## Dispatch

| Source file | Host destination |
|---|---|
| `services/webhook-receiver/quadlet/webhook-receiver.container` | `/etc/containers/systemd/webhook-receiver.container` |
| `services/webhook-receiver/quadlet/webhook-receiver.container.d/10-resources.conf` | `/etc/containers/systemd/webhook-receiver.container.d/10-resources.conf` |
| `services/webhook-receiver/systemd/webhook-receiver.socket` | `/etc/systemd/system/webhook-receiver.socket` |
| `services/webhook-receiver/systemd/webhook-receiver.socket.d/10-hardening.conf` | `/etc/systemd/system/webhook-receiver.socket.d/10-hardening.conf` |

## What this example demonstrates

- A single service spanning both `quadlet/` and `systemd/`. The
  payload-kind directory governs the deployment target root, not the
  service id.
- Drop-in convention: a file at `<unit>.<ext>.d/<file>.conf` is a
  drop-in for the parent unit `<unit>.<ext>`. Drop-ins are ordered
  lexicographically by filename — the `10-` prefix reserves room for
  later overrides at higher numeric prefixes.
- The drop-in shape is identical for Quadlet and native systemd.

## What this example does NOT demonstrate

- The `service.yaml` schema (see `02-variant-config-root/`).
- Host overlays adding drop-ins on top of base drop-ins (see
  `04-host-overlay/`).
- Config payload files (see `01-minimal-single-service/`).

## Try it

```bash
core-ops plan --source-repo . --host example-host
```
