# Quickstart: Native Mount Management

## Goal
Declare a native managed mount, attach a service dependency to it, and verify reconciliation, failure handling, and removal behavior.

## Example Workflow

1. Add a native `.mount` artifact for a network-backed share used by a selected service and add a minimal `[X-CoreOps]` section.
2. Reference that managed mount by its native `.mount` unit stem from the service definition.
3. If the mount is network-backed and on-demand activation is desired, add the matching `.automount` artifact.
4. Run `core-ops plan` and verify that the plan includes:
   - the generated `.mount` unit
   - the optional `.automount` unit only when explicitly requested for a network-backed mount
   - bounded target-path preparation if needed
   - path-based and explicit unit dependency semantics for the consuming service
5. Run `core-ops apply` and verify that:
   - the mount becomes active before the dependent service is treated as runnable
   - dependent service units carry the generated native dependency semantics
   - failure output is explicit when mount validation or activation fails

## Expected Native Dependency Semantics

For a service consuming a mounted host path:
- the generated service unit includes path-based dependency materialization for the consumed path via `RequiresMountsFor=`
- the generated service unit includes explicit unit dependencies where the mount or automount units must be referenced directly
- when automount is enabled, the service remains correctly ordered with both the automount and underlying mount behavior

## Failure and Recovery Checks

### Mount Activation Failure
- Confirm reconciliation reports which managed `.mount` stem failed.
- Confirm the dependent service is blocked rather than reported healthy.

### Mount Loss After Service Start
- Confirm an already-running service remains running.
- Confirm reconciliation marks the dependency as degraded or blocked.
- Confirm future starts or restarts remain prevented until the mount recovers.

### Mount Removal
- Remove the managed mount artifact from desired state.
- Confirm dependent managed services are stopped first.
- Confirm generated mount or automount units are removed only after the mount is no longer active.
- Confirm reconciliation fails explicitly if the mount remains busy or cannot be cleanly removed.

## Version Review Outcome

- This feature requires release-version-policy review because it introduces new managed mount artifacts, new generated unit dependency semantics, and new externally visible removal behavior.
- Confirmed package version outcome: `0.3.0 -> 0.4.0`.
