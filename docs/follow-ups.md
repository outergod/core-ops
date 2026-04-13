# Follow-Ups

Deferred implementation work and discoveries that should be revisited after the active spec work is complete.

## Config Change Restart Reporting Diverges From Actual Apply

- Status: Resolved in 0.8.2 — see `specs/014-config-restart-fidelity/` and `changes/014-config-restart-fidelity.md`
- Area: apply execution vs apply reporting for dependent restarts
- Discovery:
  - On `ulthar`, `core-ops apply --repo file:///var/lib/core-ops/repo --rev master` reported:
    - `container/github-actions-runner.container restarted`
    - cause: `restart required because /etc/github-actions-runner/start-runner.sh changed`
  - The target config file update was real, but the generated service did not actually restart.
  - Host evidence after apply:
    - `systemctl status github-actions-runner.service` still showed `Active: active (running) since Wed 2026-04-08 21:02:13 CEST`
    - `systemctl show github-actions-runner.service -p ActiveEnterTimestamp -p ExecMainStartTimestamp -p InvocationID` showed unchanged timestamps and invocation
    - `journalctl -u github-actions-runner.service -S -15m --no-pager` showed no restart attempt during the apply window
- Narrowed root cause:
  - The human apply report is synthesized from the deterministic reconciliation/object plan in `src/cli/report.rs`, where the container object is classified as `Restart` because a prerequisite config object changed.
  - Actual execution is driven by the lower-level `ReconciliationPlan.actions` from `src/core/planner.rs`.
  - For `QuadletType::ConfigFile` diffs, `actions_for_diff(...)` writes the config payload but does not schedule a dependent `RestartUnit` for the consuming service/container.
  - As a result, `src/io/apply.rs` never executes `systemctl restart github-actions-runner.service`, even though the report claims the restart happened.
- Impact:
  - Runtime behavior can remain stale after config changes that operators reasonably expect to trigger a restart.
  - The apply report overstates what actually happened, which undermines operator trust.
- Desired follow-up:
  - Make executable reconciliation actions include dependent restarts when config-file changes require reactivation of consuming workloads.
  - Ensure the human/machine apply output distinguishes between:
    - restart required by deterministic plan semantics
    - restart actually executed during apply
- Likely implementation direction:
  - Add dependency-aware restart scheduling for config-file changes in the executable planner path.
  - Add a regression test proving that a changed config file for a container-backed service leads to a real `RestartUnit` action and observable service restart.
  - Tighten apply reporting so terminal `restarted` status is sourced from real execution events or completed runtime actions, not only deterministic plan classification.
