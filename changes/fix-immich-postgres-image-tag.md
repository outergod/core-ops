---
change_id: fix-immich-postgres-image-tag
release_intent: patch
summary: Fix `examples/03-immich` postgres image tag (`:16` -> `:16-vectorchord0.3.0-pgvector0.8.0-pgvectors0.2.0`); the previous tag did not exist on `ghcr.io/immich-app/postgres` and prevented the canonical Immich walkthrough from applying end-to-end on a clean host.
scope: examples
release_preparation: false
---
