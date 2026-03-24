# Contract: Native Mount Artifact and Dependency Model

## Purpose
Define the desired-state contract for managed native `.mount` / optional `.automount`
artifacts and service-to-mount dependency references.

## Managed Mount Artifact Rules

A valid managed native mount artifact MUST provide:
- a native `.mount` unit name whose stem is the managed mount reference
- exactly one target path
- a native source and filesystem type
- native mount options when required
- selected-service ownership boundaries derived from the owning service definition
- optional bounded mountpoint-creation metadata through `[X-CoreOps]`
- optional `.automount` companion only for declared network-backed mounts

## Service Dependency Rules

A valid service dependency MUST:
- reference native `.mount` stems, not raw paths alone
- resolve each referenced stem within the selected services for the host
- derive consumed mounted paths from the referenced managed artifacts
- materialize dependency semantics in the generated native unit configuration

## Generated Native Dependency Semantics

For each dependent service unit:
- path-based dependency semantics MUST be emitted for consumed mounted paths via native mechanisms such as `RequiresMountsFor=`
- explicit unit ordering or requirement semantics MUST also be emitted when the generated mount or automount units need to be referenced directly, using native mechanisms such as `After=` and `Requires=`
- when automount is enabled, service dependencies MUST remain coherent with both the generated `.automount` and `.mount` units

## Validation Expectations

Reject desired state when:
- a referenced managed mount stem is missing
- two mount declarations conflict on target path or incompatible semantics
- automount is requested for a mount that is not explicitly network-backed
- the native unit stem does not match the declared `Mount` `Where=` path
- unsupported `[X-CoreOps]` fields are present
