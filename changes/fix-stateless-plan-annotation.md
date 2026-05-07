---
change_id: fix-stateless-plan-annotation
release_intent: patch
summary: Stateless `plan` and `apply` no longer flag a healthy host as "recovery from failed initial apply"; introduce `ApplyRunDisplayState::Stateless` so the rendered header carries only the `(stateless)` path-based provenance prefix when the host has converged objects but no `/var/lib/core-ops/` baseline. Empty-host invocations continue to render `(first run)` as before.
scope: cli
release_preparation: false
---
