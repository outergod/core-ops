# Research: Native Mount Management

## Decision: Represent mounts as named desired-state declarations

- **Decision**: Model each managed mount as a native `.mount` artifact whose unit stem is the managed reference, with a minimal `[X-CoreOps]` section for bounded mountpoint creation only; dependent services reference native `.mount` stems rather than raw paths alone.
- **Rationale**: Stable identities produce clearer validation, better diagnostics, and cleaner host overrides than path-only matching. The target path remains part of the declaration, but it is no longer the only key.
- **Alternatives considered**: Path-only dependencies (simpler but brittle when paths change or collide), dual path-and-name references (more ambiguous and increases validation complexity).

## Decision: Materialize dependencies using both path-based and explicit unit semantics

- **Decision**: Generate native unit dependencies in two forms: path-based dependency materialization for consumed mounted paths via mechanisms such as `RequiresMountsFor`, and explicit unit ordering or requirement links such as `After=` and `Requires=` when the generated mount or automount units must be referenced directly.
- **Rationale**: This matches native systemd behavior and covers both normal mount consumption and automount-specific ordering. Path-only dependencies underspecify automount behavior; unit-only dependencies lose the direct relationship between a service and the consumed path.
- **Alternatives considered**: Path-only semantics (insufficient for explicit automount ordering), explicit-unit-only semantics (harder to reason about consumed paths and more fragile under target-path evolution).

## Decision: Limit automount support to explicitly declared network-backed mounts

- **Decision**: Support automount units only when a mount declaration explicitly opts in for a network-backed mount such as NFS.
- **Rationale**: The current blocked use case is NAS-backed storage. Limiting automount to declared network-backed mounts keeps the feature bounded and avoids turning all mount behavior into a generic lifecycle configuration surface.
- **Alternatives considered**: No automount support in this iteration (simpler but misses a valid operator need), unrestricted automount support (too broad for the current scope).

## Decision: Treat bounded directory preparation as part of mount reconciliation, not general configuration management

- **Decision**: Allow reconciliation to create declared mount target paths and required parent directories, and optionally enforce owner, group, and mode only for service-consumed directories associated with managed mounts.
- **Rationale**: Mount activation often requires a valid target path. Allowing bounded preparation solves that need while preserving the project constraint against becoming a generic directory-management system.
- **Alternatives considered**: No preparation at all (operator friction for common NFS workflows), full filesystem metadata management (out of scope and violates feature boundaries).

## Decision: Keep running services in place when a required mount later disappears

- **Decision**: If a required mount becomes unavailable after a dependent managed service is already running, leave the service in place, mark it degraded or blocked, and prevent future starts or restarts until the mount recovers.
- **Rationale**: This preserves safe defaults and avoids converting a storage outage into additional service churn. It also keeps failure semantics explicit.
- **Alternatives considered**: Automatically stop or restart dependent services (more disruptive), make post-failure behavior fully configurable in this iteration (adds unnecessary scope).

## Decision: Remove mounts conservatively and fail explicitly if they remain busy

- **Decision**: When a previously managed mount is removed from desired state, stop dependent managed services first, then remove generated mount or automount units only after the mount is no longer active. If the mount remains busy or cannot be cleanly deactivated, reconciliation fails explicitly.
- **Rationale**: This preserves clear ownership and safe teardown semantics without forcing unmount or silently leaving stale managed units behind.
- **Alternatives considered**: Leave active mounts behind as unmanaged state (ambiguous ownership), require fully manual operator cleanup before any reconciliation can proceed (overly rigid).

## Decision: Verify mount lifecycle through native systemd and host path checks

- **Decision**: Verification should combine native unit state with host-path verification: a managed mount is considered active and verified only when the generated mount or automount units are in the expected state and the declared target path is mounted and usable for dependent services.
- **Rationale**: Unit state alone can miss path-level problems; path checks alone can miss incorrect unit wiring. Combining both keeps the behavior explicit and testable.
- **Alternatives considered**: Unit-state-only verification (too weak), path-state-only verification (misses native dependency intent).

## Decision: Treat this feature as a minor version review candidate

- **Decision**: Record this feature as requiring release-version-policy review with an expected minor version update from `0.3.0` to `0.4.0`, subject to confirmation after implementation.
- **Rationale**: Native mount artifacts, new dependency semantics in generated units, and new removal behavior materially affect externally observable reconciliation behavior and compatibility expectations, but they do not currently justify a major break under the pre-1.0 policy.
- **Alternatives considered**: Patch-level treatment (too small for the externally visible behavior change), major version review (not currently justified by the scope).
