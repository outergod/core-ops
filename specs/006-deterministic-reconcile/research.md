# Research: Deterministic Reconciliation

## Decision 1: Use normalized three-way comparison as the only planner input model

- Decision: Compute reconciliation decisions from normalized `desired`, `last_applied`, and `actual` state snapshots for the same managed scope, and fall back to desired-vs-actual only when no compatible `last_applied` snapshot exists.
- Rationale: Three-way comparison is the minimum model that can distinguish expected change from operator drift, stale residue, and tolerated runtime variance. It also preserves historical continuity for rollback and explainable planning.
- Alternatives considered:
  - Desired versus actual only: rejected because it cannot reliably distinguish external drift from CoreOps-authored change.
  - Resource-specific ad hoc planners: rejected because they weaken determinism and make explanation/reporting inconsistent.

## Decision 2: Keep normalization explicit and per-resource-kind

- Decision: Define canonical normalization rules per managed resource kind and compare only normalized representations during planning, diffing, and convergence checks.
- Rationale: Deterministic planning depends on stable ordering and the removal of non-semantic formatting differences without hiding material drift. Resource-specific rules are necessary because units, rendered artifacts, Quadlet resources, and mount semantics differ materially.
- Alternatives considered:
  - Raw file-text comparison: rejected because formatting and ordering noise would create false drift.
  - Fully generic normalization for all resource kinds: rejected because it would either leak semantics or overfit to the lowest common denominator.

## Decision 3: Maintain a separate semantic dependency graph rather than relying on systemd ordering alone

- Decision: Build a minimal explicit dependency graph in CoreOps for planning, rollback ordering, cycle detection, and explanations, while still materializing the necessary runtime directives into native units.
- Rationale: Systemd remains the runtime executor, but CoreOps must own semantic ordering and causality to satisfy deterministic planning, structured diff output, and rollback guarantees.
- Alternatives considered:
  - Infer ordering only from generated systemd directives: rejected because it is not sufficiently explainable and obscures semantic cycles before execution.
  - Model a fully general graph for arbitrary host resources: rejected because this iteration is intentionally scoped to CoreOps-managed single-node resources.

## Decision 4: Use managed objects as graph nodes and preserve a stable canonical order

- Decision: Model graph nodes at the managed-object level for generated units, Quadlet resources, managed mounts or automounts, and rendered artifacts, with stable node identity and deterministic topological ordering plus a canonical lexical tie-breaker.
- Rationale: Managed-object nodes align with existing CoreOps reconciliation boundaries and are the smallest units that still support clear action explanations and rollback ordering.
- Alternatives considered:
  - Model every field as a node: rejected as too complex for the current scope.
  - Model only coarse bundles such as "service stack": rejected because it weakens drift and restart reasoning.

## Decision 5: Classify decisions as create, update, delete, replace, no-op, or blocked

- Decision: Keep the planner's first-class decision surface aligned with the spec taxonomy and emit those classifications in machine-readable and human-readable output.
- Rationale: A stable action taxonomy makes planning, execution ordering, auditability, and failure explanation coherent across resource kinds.
- Alternatives considered:
  - Collapse replace into update or delete+create: rejected because disruption semantics and dependent impact need to be explicit.
  - Hide blocked decisions until execution time: rejected because blockage is a planning outcome and must be visible before side effects occur.

## Decision 6: Treat rollback as ordinary reconciliation against a retained successful snapshot

- Decision: Implement rollback by selecting a previously successful retained revision as the new desired target and re-running the same three-way planner with the retained `last_applied` snapshot and current `actual` state.
- Rationale: This preserves one reconciliation model, keeps rollback auditable, and avoids brittle inverse imperative actions.
- Alternatives considered:
  - Store inverse actions during forward apply: rejected because it is not durable across drift and partial failure.
  - Reconstruct rollback only from current desired and historical desired: rejected because it loses the required successful-applied boundary.

## Decision 7: Define success strictly at the post-verify convergence boundary

- Decision: A revision becomes successfully applied only after all planned side effects complete and post-apply verification confirms convergence for the managed scope.
- Rationale: This keeps `last_applied` trustworthy for later three-way planning and rollback, and prevents partial or merely attempted revisions from being treated as known-good state.
- Alternatives considered:
  - Mark success after side effects complete: rejected because post-apply drift or blocked runtime state would poison future planning.
  - Mark success per object: rejected because it would complicate rollback eligibility and historical continuity in this iteration.

## Decision 8: Bound rollback eligibility by retained successful history

- Decision: Retain only a bounded window of successful applied snapshots for rollback eligibility, and fail rollback safely when the necessary retained snapshot is unavailable.
- Rationale: Bounded retention keeps persisted state manageable while preserving predictable rollback semantics.
- Alternatives considered:
  - Infinite rollback history: rejected because it complicates retention and compatibility obligations.
  - Manual operator pinning only: rejected because it weakens deterministic safety guarantees.

## Decision 9: Detect non-convergence with bounded retries and explicit signatures

- Decision: Track repeated failure and oscillation using a small fixed retry budget keyed by affected object set and failure pattern or oscillation signature, then stop automatic retry and require intervention.
- Rationale: Deterministic reconciliation must surface instability instead of hiding it behind indefinite retry. A fixed policy is simpler and more testable than resource-specific retry logic in this iteration.
- Alternatives considered:
  - Retry until desired revision changes: rejected because it can loop indefinitely.
  - Stop after the first failure: rejected because it overreacts to transient conditions.
  - Resource-specific retry policies now: rejected because it adds complexity before the core model is proven.

## Decision 10: Use one canonical structured diff model for plan and apply reporting

- Decision: Produce a machine-readable structured diff model that carries object identity, action classification, dependency context, revision identifiers, rationale, and semantic differences, and derive human-readable rendering from that same model.
- Rationale: This preserves consistency between human and machine outputs and supports later humane UI and agent workflows without a second reporting path.
- Alternatives considered:
  - Human-only summaries: rejected because they are not sufficient for automation or auditability.
  - Separate schemas for planning and apply output: rejected because they would drift and complicate tests.

## Decision 11: Persist enough state to reconstruct planning deterministically, not full execution history

- Decision: Persist the successful applied snapshot, revision identifiers, convergence result summary, and enough dependency metadata to reconstruct later three-way reasoning deterministically; persist failed-run diagnostics without promoting them to `last_applied`.
- Rationale: This is the minimum persisted state needed for rollback, drift explanation, and reproducible planning without turning the status file into a general event journal.
- Alternatives considered:
  - Persist only revision identifiers: rejected because it is insufficient for rollback planning and drift analysis.
  - Persist a full append-only history log in this iteration: rejected because it exceeds the feature scope.

## Decision 12: Keep implementation within existing CoreOps module boundaries

- Decision: Extend existing `core`, `io`, and `cli` modules rather than introducing a separate reconciliation subsystem.
- Rationale: The constitution favors minimal abstractions and regenerable structure. Existing boundaries already map well onto pure planning logic, side-effectful IO, and presentation.
- Alternatives considered:
  - Introduce a new planner service layer or repository abstraction: rejected because the feature does not require a new architectural tier.
