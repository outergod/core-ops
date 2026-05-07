---
change_id: fix-immich-server-db-password
release_intent: patch
summary: Wire `immich-db-password` Podman secret + `DB_PASSWORD_FILE` env var into `examples/03-immich/services/immich-server/quadlet/immich-server.container`; without these, `immich-server` could never authenticate to the Postgres database started by `immich-database.container` and entered a restart loop with `password authentication failed for user "immich"`.
scope: examples
release_preparation: false
---
