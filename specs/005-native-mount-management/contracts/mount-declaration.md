# Contract: Mount Declaration and Dependency Model

## Purpose
Define the desired-state contract for managed mounts and service-to-mount dependency references.

## Mount Declaration Rules

A valid mount declaration MUST provide:
- a stable declaration identity
- exactly one target path
- a native source and filesystem type
- native mount options when required
- selected-service ownership boundaries
- optional bounded target-path preparation metadata
- optional automount intent only for declared network-backed mounts

## Service Dependency Rules

A valid service dependency MUST:
- reference mount declaration identities, not raw paths alone
- resolve each referenced identity within the selected services for the host
- derive consumed mounted paths from the referenced declarations
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
