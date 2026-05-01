# Example 04 — Host Overlay

A base service with one container and one config file, plus a host
that contributes both a drop-in addition and a `config/` whole-file
replacement. Demonstrates the per-host overlay shape and the merge
order rule (base service drop-ins lex-sorted, then host drop-ins
lex-sorted; whole-file `config/` entries fully replace base files).

This example backs spec 016 FR-005 (host overlays MAY contribute
drop-ins and whole-file replacements but MUST NOT introduce base
units), FR-014 (drop-in merge order), and the host-overlay class of
FR-023.

## Tree

```text
04-host-overlay/
├── services/
│   └── node-exporter/
│       ├── quadlet/
│       │   └── node-exporter.container
│       └── config/
│           └── node-exporter.env
└── hosts/
    └── host-a/
        ├── host.yaml
        └── node-exporter/
            ├── quadlet/
            │   └── node-exporter.container.d/
            │       └── 30-listen-port.conf
            └── config/
                └── node-exporter.env
```

No `service.yaml`; `config-root` defaults to `node-exporter`.

## Dispatch (host = `host-a`)

| Source file | Host destination |
|---|---|
| `services/node-exporter/quadlet/node-exporter.container` | `/etc/containers/systemd/node-exporter.container` |
| `hosts/host-a/node-exporter/quadlet/node-exporter.container.d/30-listen-port.conf` | merged into `/etc/containers/systemd/node-exporter.container.d/`, lex-sorted after any base service drop-ins |
| `hosts/host-a/node-exporter/config/node-exporter.env` | `/etc/node-exporter/node-exporter.env` (replaces the base service's file) |

The base service ships its own `config/node-exporter.env`; the host's
`config/node-exporter.env` replaces it byte-for-byte at the same
destination. There is no merge or template at the file level — config
overlays are whole-file replacements only in layout v1.

## What this example demonstrates

- Host overlay shape: per-service subdirectory under `hosts/<host-id>/`
  mirroring the service's payload-kind tree.
- Drop-in addition from a host: a `.conf` under `<unit>.<ext>.d/` that
  refines the base unit. The numeric prefix `30-` lex-sorts after any
  base service drop-ins prefixed `10-` or `20-`.
- Whole-file replacement: a host's `config/<rel>` overrides the base
  service's `config/<rel>` at the same path under `/etc/<config-root>/`.
- Forbidden form (NOT shown but enforced by the parser): a base unit
  file directly at `hosts/host-a/node-exporter/quadlet/<unit>.<ext>` is
  rejected with a diagnostic — the host overlay can only refine, not
  introduce.

## What this example does NOT demonstrate

- The `service.yaml` schema (see `02-variant-config-root/`).
- Multi-unit services (see `03-multi-unit-with-dropins/`).
- Diagnostics on legacy or invalid layouts (see the integration
  tests in `tests/integration/test_source_repo_layout.rs`).

## Try it

```bash
core-ops plan --source-repo . --host host-a
```
