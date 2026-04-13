---
change_id: 014-config-restart-fidelity
release_intent: patch
summary: Fix config-file changes not triggering dependent container restarts
scope: planner
release_preparation: false
---

Config-file changes, removals, and additions (when the container was already running)
now correctly schedule `RestartUnit` actions for dependent containers during `apply`.
Previously, the planner emitted only `WriteQuadlet` for `ConfigFile` diffs and never
consulted the dependency graph, leaving services silently running with stale configuration
while the apply report incorrectly claimed they had restarted.
