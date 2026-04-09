# Follow-Ups

Deferred implementation work and discoveries that should be revisited after the active spec work is complete.

## CLI Revision Input

- Status: Deferred until after current spec implementation
- Area: Git revision resolution in `core-ops plan` / `core-ops apply`
- Discovery:
  - Human-readable output shortens target revisions to 8 characters for display, for example `454ac5f1`.
  - Those short revisions are not reliably accepted as `--rev` input.
  - Current loader behavior in `src/io/repo.rs` uses `git fetch origin <rev>` followed by checkout from `FETCH_HEAD`, so `--rev` currently behaves like a fetchable ref rather than a general Git-resolvable revision.
  - Full SHAs, branch names, tags, and supported revision expressions such as `main~1` work; short SHAs may fail even when unambiguous in the source repository.
- Desired follow-up:
  - Make short commit IDs accepted as `--rev` input when they are unambiguous in the source repository.
  - Keep displayed short revisions and accepted CLI revision syntax aligned enough that operator expectations are not violated.
- Likely implementation direction:
  - Resolve candidate revisions with Git after clone/fetch instead of treating all plain inputs as fetch refspecs only, or add an explicit short-SHA resolution path before checkout.

## Config Change Restart Reporting Diverges From Actual Apply

- Status: Deferred follow-up after distribution-readiness merge
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
