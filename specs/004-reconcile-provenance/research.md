# Research: Provenance and Reconciliation Revision Tracking

## Decision: Use a single canonical persisted status snapshot file

- **Decision**: Persist provenance in one canonical local status file that is the authoritative source for this iteration; CLI and other surfaces mirror that file instead of maintaining separate persisted state.
- **Rationale**: This matches the spec's derivative-state rule, minimizes split-brain risk between interfaces, and gives operators and agents a single machine-readable source for current state and last reconciliation outcome.
- **Alternatives considered**: Independent CLI cache plus file (risks divergence), journal-only storage (conflicts with summary-only scope), logs as canonical state (poor restart-time reconstructibility).

## Decision: Use atomic snapshot replacement semantics for persisted provenance

- **Decision**: Treat persisted provenance as an atomic full-snapshot document written through replace-in-place semantics so readers either observe the old valid snapshot or the new valid snapshot, never a partial transition.
- **Rationale**: The spec requires complete-snapshot readability, interruption safety, and explicit validity semantics. Atomic replacement is the simplest approach that satisfies those properties on the target platform.
- **Alternatives considered**: In-place mutation (partial-read risk), append-only event log (out of scope for this iteration), multi-file state shards (harder to validate atomically).

## Decision: Invalid or unsupported persisted state is ignored as absent

- **Decision**: Readers validate completeness and schema support before interpreting persisted provenance; invalid, partial, or unsupported snapshots are ignored and treated as absent state.
- **Rationale**: This preserves safe defaults and avoids presenting corrupted provenance as truth.
- **Alternatives considered**: Best-effort partial parsing (ambiguous semantics), hard failure on startup (unnecessarily brittle for a derivative cache).

## Decision: Model provenance as three distinct domains

- **Decision**: Represent controller provenance as identity data, desired-state provenance as observational data, and reconciliation provenance as operational state.
- **Rationale**: The split aligns with the spec, keeps the data model legible, and prevents fields with different semantics from being conflated in status output or persisted state transitions.
- **Alternatives considered**: Single flat provenance record without domain boundaries (harder to reason about), controller+desired-state merged under one revision block (blurs identity vs observation).

## Decision: Scope this iteration to current state plus last reconciliation outcome

- **Decision**: Persist and expose only the current provenance snapshot and the most recent reconciliation outcome; do not introduce bounded history or sequence-analysis features.
- **Rationale**: This satisfies the feature goal while keeping the state model minimal and derivative.
- **Alternatives considered**: Bounded event journal (adds schema and migration surface), full historical analysis (explicitly out of scope).

## Decision: Add a dedicated persisted provenance contract alongside CLI surface documentation

- **Decision**: Define one contract for the canonical status file shape and one contract for the CLI/status surface that reflects it.
- **Rationale**: The feature exposes both a persisted machine-readable artifact and operator-facing command output; both need stable expectations for tests and future changes.
- **Alternatives considered**: CLI-only contract (insufficient for the canonical file), file-only contract (underspecifies operator access path).
