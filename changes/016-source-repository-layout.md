---
change_id: 016-source-repository-layout
release_intent: major
summary: Formalize source repository layout; remove legacy parser; add core-ops skill install
scope: source-repo
release_preparation: false
---

Formalizes the on-disk shape of CoreOps source repositories around payload-kind directories
(`quadlet/`, `systemd/`, `config/`) at each service root, an optional `service.yaml` declaring
`config-root` for variant services, and a host overlay tree that mirrors the service shape
directly under `hosts/<host-id>/<svc-id>/`. Drop-ins keep systemd-native filesystem parity
(`<unit>.<ext>.d/<file>.conf`).

**Breaking changes**: the legacy `quadlets/` and `quadlet-overrides/` parsers are removed.
Source repositories using the legacy layout are rejected at load time with a diagnostic that
points at `scripts/migrate-legacy-source-repo.sh` for mechanical migration. The `overrides/`
segment under `hosts/<host-id>/` is also removed; host overlays now mirror the service shape
directly.

**New CLI surface**: `core-ops skill install [--global] [--print]` writes a vendor-neutral
agent-skill bundle to `.agents/skills/core-ops-source-repo/` (per agentskills.io). The
subcommand is independent of `core-ops init`.

**State snapshot**: the persisted controller status snapshot gains a `layout-version: "1"`
field so future revisions can detect which layout produced a given snapshot.

Reference example source repositories ship in-tree at
`specs/016-source-repository-layout/examples/` covering minimal, variant-config-root,
multi-unit, and host-overlay shapes.
