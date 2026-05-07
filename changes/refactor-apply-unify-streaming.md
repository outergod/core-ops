---
change_id: refactor-apply-unify-streaming
release_intent: patch
summary: Unify init'd and stateless apply dispatch behind a single `ApplyTarget` abstraction so `core-ops apply --source-repo PATH` gains the streaming + interactive output that `core-ops apply` (init'd) has had since spec/006. Previously stateless apply printed a single wall-of-text summary at the end regardless of TTY state because `apply_with_report_stateless` was a separate batch-only entry point that never picked up the spec/006 streaming/interactive variants.
scope: cli
release_preparation: false
---
