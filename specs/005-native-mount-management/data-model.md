# Data Model: Native Mount Management

## Entities

### Mount Declaration
- **Purpose**: Define one managed host mount as named desired state.
- **Fields**:
  - `id` (string, stable declaration identity)
  - `target_path` (absolute path)
  - `source` (string, native mount source)
  - `fstype` (string, native filesystem or network type)
  - `mount_options` (list of strings)
  - `network_backed` (boolean)
  - `automount` (boolean, only valid when `network_backed = true`)
  - `verification_mode` (`unit_and_path` by default)
  - `ownership_scope` (selected service identities allowed to reference it)
  - `prepared_directory` (optional Prepared Target Path)
- **Rules**:
  - `id` must be unique within the evaluated host desired state.
  - `target_path` must be unique across mount declarations for the host.
  - `automount = true` is valid only for explicitly declared network-backed mounts.
  - The declaration remains systemd-native and must map to generated `.mount` and optional `.automount` units.

### Mount Dependency
- **Purpose**: Describe a service's dependency on one or more mount declarations.
- **Fields**:
  - `service_id` (string)
  - `mount_ids` (list of Mount Declaration ids)
  - `consumed_paths` (list of absolute paths derived from referenced mounts)
  - `path_dependency_mode` (`requires_mounts_for`)
  - `unit_dependency_mode` (`after_and_requires` when explicit unit references are needed)
- **Rules**:
  - Every `mount_id` must resolve to a selected, owned Mount Declaration.
  - Consumed paths must be consistent with the referenced mount target paths.
  - Generated service units must include both path-based and explicit unit dependencies where required by the mount or automount configuration.

### Prepared Target Path
- **Purpose**: Constrain host path creation or metadata enforcement needed before mount activation.
- **Fields**:
  - `path` (absolute path)
  - `create_if_missing` (boolean)
  - `owner` (optional string)
  - `group` (optional string)
  - `mode` (optional string)
  - `service_consumed` (boolean)
- **Rules**:
  - Applies only to the declared mount target path and required parent directories.
  - Owner, group, and mode may only be enforced when the directory is explicitly service-consumed.
  - Must not expand into general directory management beyond bounded mount preparation.

### Generated Native Unit Set
- **Purpose**: Represent the native systemd artifacts emitted from one mount declaration and its consumers.
- **Fields**:
  - `mount_unit_name` (string)
  - `automount_unit_name` (optional string)
  - `service_dependency_edits` (list of generated unit dependency changes)
  - `removal_candidates` (list of units to remove when desired state drops the declaration)
- **Rules**:
  - A mount declaration always produces one `.mount` unit.
  - It optionally produces one `.automount` unit when automount is explicitly enabled for a network-backed mount.
  - Dependent service units must reflect both path-based and explicit unit dependency semantics.

### Mount Reconciliation Result
- **Purpose**: Capture the operator-visible result of reconciling one mount declaration.
- **Fields**:
  - `mount_id` (string)
  - `validation_status` (`valid` | `invalid`)
  - `activation_status` (`not_applied` | `active` | `degraded` | `failed` | `removing` | `removed`)
  - `verification_status` (`verified` | `unverified` | `not_applicable`)
  - `dependent_service_effect` (`none` | `blocked` | `degraded` | `stopped_for_removal`)
  - `failure_reason` (optional string)
  - `removal_result` (`not_requested` | `removed` | `busy` | `failed`)
- **Rules**:
  - Failure and removal outcomes must remain explicit.
  - If the mount disappears after a service is already running, dependent service effect is `degraded` or `blocked`, not forced stop.
  - If desired state drops the mount and it remains busy, `removal_result` must be `busy` or `failed` rather than silently ignored.

## Relationships

- One **Mount Declaration** may be referenced by zero or more **Mount Dependencies**.
- One **Mount Declaration** may include zero or one **Prepared Target Path**.
- One **Mount Declaration** produces one **Generated Native Unit Set**.
- One **Mount Reconciliation Result** is recorded per Mount Declaration per reconciliation attempt.
- **Mount Dependencies** determine the service-side edits in the **Generated Native Unit Set**.

## State Transitions

### Mount Lifecycle
- `not_applied -> active` after successful mount activation and verification.
- `not_applied -> failed` on validation or activation failure.
- `active -> degraded` when the mount later becomes unavailable while a dependent service remains running.
- `degraded -> active` when a later reconciliation successfully re-verifies the mount.
- `active -> removing` when desired state drops the declaration.
- `removing -> removed` when dependent managed services are stopped, the mount is no longer active, and generated units are removed.
- `removing -> failed` when teardown cannot complete.

### Service Dependency Effects
- `none -> blocked` when a required mount is not yet active for a service that has not started.
- `none -> degraded` when a required mount disappears after the service is already running.
- `blocked -> none` when the mount becomes active and verified.
- `degraded -> none` when the mount recovers and the dependency verifies again.
- `none -> stopped_for_removal` when a mount is intentionally removed and dependent managed services are stopped first.

## Validation Rules

- Mount declaration identities must be unique.
- Two mount declarations must not claim the same target path with conflicting definitions.
- Service references to mount ids outside selected-service ownership boundaries are invalid.
- Automount declarations without a corresponding network-backed mount declaration are invalid.
- Prepared target paths must remain within the bounded mount target path contract.
- Removal may proceed only after dependent managed services are stopped and the mount is no longer active.
