# Research: Systemd-Managed Host Agent

## Decision: Systemd automation mode

**Decision**: Ship both a oneshot service and a timer that triggers it.
**Rationale**: Aligns with Fedora CoreOS operational norms and keeps unattended
execution explicit and inspectable.
**Alternatives considered**: Long-running daemon only; timer-only without
standalone service.

## Decision: Artifact ordering

**Decision**: Volume → Container → Socket ordering for reconciliation.
**Rationale**: Volumes must exist before containers reference them; sockets
typically depend on services or containers.
**Alternatives considered**: Socket-first ordering; container-first ordering.

## Decision: Verification behavior

**Decision**: Verify via systemd unit state checks (active/enabled where
applicable) for each artifact type.
**Rationale**: Uses native systemd primitives without custom probes and provides
consistent, inspectable outcomes.
**Alternatives considered**: File existence-only verification; runtime probes.
