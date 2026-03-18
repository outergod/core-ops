MVP architecture decisions:
- Rust workspace or single crate
- domain types: DesiredState, ObservedState, Diff, Plan, Outcome
- pure core modules: model, validate, diff, plan
- effect modules: git, filesystem, systemd, quadlet, process
- orchestrator: reconcile()
- CLI modes: plan, apply, verify
- tests: unit + scenario + property tests for core logic
