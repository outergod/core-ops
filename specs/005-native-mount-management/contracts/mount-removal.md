# Contract: Managed Mount Removal Behavior

## Purpose
Define the expected behavior when desired state drops a previously managed mount declaration.

## Removal Sequence

When desired state removes a managed mount declaration, reconciliation MUST:
1. identify dependent managed services
2. stop those dependent managed services first
3. deactivate the managed mount or automount only after dependencies are no longer active
4. remove the generated mount or automount units only after the mount is no longer active
5. report the removal outcome explicitly

## Failure Rules

- If the mount remains busy, reconciliation MUST fail explicitly.
- If generated units cannot be cleanly removed, reconciliation MUST fail explicitly.
- Reconciliation MUST NOT silently leave a previously managed mount as ambiguous unmanaged state.
- Reconciliation MUST NOT force unmount destructive behavior by default.

## Automount-Specific Rules

When automount is enabled for a network-backed mount:
- dependent service units must continue to carry path-based dependency semantics for consumed paths
- explicit unit dependencies may reference the generated automount or underlying mount units where required for correct native ordering and activation behavior
- removal must treat the `.automount` and `.mount` units as part of the same managed declaration and remove them coherently
