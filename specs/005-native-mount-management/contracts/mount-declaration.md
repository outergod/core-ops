# Contract: Native Mount Artifact and Dependency Model

## Purpose
Define the desired-state contract for managed native `.mount` / optional `.automount`
artifacts and service-to-mount dependency references.

## Managed Mount Artifact Rules

A valid managed native mount artifact MUST provide:
- a stable identity through embedded `[X-CoreOps]` metadata
- exactly one target path
- a native source and filesystem type
- native mount options when required
- selected-service ownership boundaries
- optional bounded target-path preparation metadata
- optional `.automount` companion only for declared network-backed mounts

## Service Dependency Rules

A valid service dependency MUST:
- reference mount declaration identities, not raw paths alone
- resolve each referenced identity within the selected services for the host
- derive consumed mounted paths from the referenced managed artifacts
- materialize dependency semantics in the generated native unit configuration

## Generated Native Dependency Semantics

For each dependent service unit:
- path-based dependency semantics MUST be emitted for consumed mounted paths via native mechanisms such as `RequiresMountsFor=`
- explicit unit ordering or requirement semantics MUST also be emitted when the generated mount or automount units need to be referenced directly, using native mechanisms such as `After=` and `Requires=`
- when automount is enabled, service dependencies MUST remain coherent with both the generated `.automount` and `.mount` units

## Validation Expectations

Reject desired state when:
- a referenced mount declaration identity is missing
- two mount declarations conflict on target path or incompatible semantics
- automount is requested for a mount that is not explicitly network-backed
- bounded directory preparation exceeds the declared service-consumed target scope
