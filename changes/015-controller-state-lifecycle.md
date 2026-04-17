---
change_id: 015-controller-state-lifecycle
release_intent: major
summary: Add controller lifecycle state management and core-ops init command
scope: controller
release_preparation: false
---

Adds explicit lifecycle state management to all reconciliation commands and introduces
`core-ops init` as a required one-time setup step.

**Breaking changes**: `--repo` and `--rev` flags are removed from `plan`, `apply`,
`agent`, and `explain`. Repository and ref are now read from the persisted state
written by `core-ops init <repository> <ref>`. Operators must run `core-ops init`
once before any reconciliation command will function.

New `detached` lifecycle state is written after a successful rollback. In Detached
state, `agent` and `apply` reconciliation are paused; `plan` continues to operate
and annotates its output with the detached revision. Use
`core-ops init --force <repository> <ref>` to re-attach.

Corrupt state files now produce a distinct named error (with file path and recovery
hint) instead of silent absent-state treatment.
