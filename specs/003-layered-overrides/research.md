# Research: Layered Overrides for Reusable Desired State

## Decision: Host identity selection

- **Decision**: Determine host identity from OS hostname by default, with an explicit CLI/env override.
- **Rationale**: Avoids circular dependency on repo contents and keeps evaluation deterministic.
- **Alternatives considered**: Host identity derived from host.yaml only (circular), required explicit override only (adds operator burden).

## Decision: Repository layout

- **Decision**: Use `services/` for shared base artifacts and `hosts/<host>/overrides/` for host-specific drop-ins; `hosts/<host>/host.yaml` declares service selection.
- **Rationale**: Mirrors native drop-in semantics and keeps base artifacts reusable.
- **Alternatives considered**: Single flat quadlets directory (no reuse), custom templating directories (violates constraints).

## Decision: Drop-in ordering

- **Decision**: Apply native lexicographic ordering for drop-ins, with host overrides layered after base drop-ins.
- **Rationale**: Matches systemd/Quadlet expectations and ensures deterministic evaluation.
- **Alternatives considered**: Fixed override order without filename ordering (non-native), single override file (too restrictive).

## Decision: Validation rules

- **Decision**: Fail evaluation when a host selects undefined services or overlays target nonexistent artifacts.
- **Rationale**: Prevents silent drift and keeps failures explicit.
- **Alternatives considered**: Ignore missing services (risk of hidden misconfigurations).
