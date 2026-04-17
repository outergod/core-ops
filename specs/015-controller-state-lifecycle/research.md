# Phase 0 Research: Controller State Model and Lifecycle

**Feature**: 015-controller-state-lifecycle  
**Date**: 2026-04-14

---

## 1. StateError::Corrupt — Distinguishing Corrupt vs Absent State Files

**Decision**: Add a `Corrupt(String)` variant to `StateError` in `src/core/errors.rs`.

**Rationale**: `read_persisted_state` currently calls `parse_persisted_state_text` and returns `Ok(None)` for both absent files (I/O `NotFound`) and unparseable files. The spec requires absent and corrupt to produce distinguishable errors. Adding a dedicated variant is the minimal change: it does not alter the success path, requires no new traits, and gives callers a typed arm to match.

**Alternatives considered**:
- Return `Ok(None)` for corrupt and detect externally — rejected; callers cannot distinguish the cases without re-reading the file.
- A separate `read_persisted_state_strict` function — rejected; creates a parallel API that callers must know to choose; the spec says all commands must fail on corrupt state, so the strict version becomes the only correct one.

---

## 2. Caller Audit for `read_persisted_state`

**Current callers and required changes**:

| Location | Current pattern | Required change |
|---|---|---|
| `src/io/state.rs:137` | `read_persisted_state(path)?` in `persist_never_run_state` | Must propagate `Corrupt` as an error (already returns `Err`) |
| `src/io/state.rs:172` | `read_persisted_state(path)?` in `persist_in_progress_state` | Same — already propagates |
| `src/cli/plan.rs:133` | `read_persisted_state(&state_path)` → maps error | Must map `Corrupt` arm with actionable message |
| `src/cli/agent.rs:62` | `.ok().flatten()` — discards error post-apply for audit event | May remain `.ok().flatten()` post-apply only; pre-apply check (new code) must handle `Corrupt` |
| `src/main.rs:161` | `.ok().flatten()` — discards error for audit event after apply | Same as agent.rs — post-apply only; safe to discard after the fact |
| `src/cli/apply.rs:821` | `read_persisted_state(path)` | Inspect usage; must propagate corrupt if used for pre-apply guard |
| `src/cli/explain.rs:34,148` | `read_persisted_state` twice; error mapped to CoreError | Both sites must handle `Corrupt` with distinct message |
| `src/cli/status.rs:15` | `match read_persisted_state(path)` | Must add `Err(StateError::Corrupt(...))` arm; status should report corrupt state rather than silently treating as absent |

**Key rule**: `.ok().flatten()` is only safe for post-apply audit emission (best-effort). Any pre-action use that gates command execution must handle `Corrupt` explicitly.

**Decision**: The `read_persisted_state` function itself changes: when file exists but `parse_persisted_state_text` returns `None`, return `Err(StateError::Corrupt(path_display))` instead of `Ok(None)`. All callers that currently use `.ok().flatten()` for pre-action guards must be updated; post-apply best-effort audit calls may retain `.ok().flatten()`.

---

## 3. Detached Flag Schema Addition

**Decision**: Add `#[serde(default)] pub detached: bool` to `PersistedProvenanceState` in `src/core/types.rs`.

**Rationale**: `PersistedProvenanceState` is serde-deserialized. Adding a field with `#[serde(default)]` means existing state files that lack the key deserialize as `false` (not detached), preserving backward compatibility with zero migration cost. The spec explicitly requires: "existing snapshots without the flag are treated as not detached".

**Alternatives considered**:
- `Option<bool>` — rejected; `None` and `false` are semantically identical here; `bool` is simpler.
- New schema version — rejected; a single optional field does not warrant a schema version bump; `is_supported_schema()` stays at the existing version.

---

## 4. `init` Command — Ref Validation Approach

**Decision**: Validate `<ref>` by attempting to resolve it in the given repository using `git ls-remote` or the existing `git2` / shell-out Git infrastructure, then checking that the resolved ref name starts with `refs/heads/` or `refs/tags/`. Bare commit hashes (40-hex strings) and other arbitrary refs must be rejected with a clear error.

**Rationale**: The spec says only branch names and tag names are valid requested refs. The codebase already resolves refs before apply (see `src/io/repo.rs`). The same resolution logic can be reused; the `init` command checks the ref type, not just resolvability.

**Pattern for detection**:
- Branch: `refs/heads/<name>` — symbolic ref points to a branch
- Tag: `refs/tags/<name>` — symbolic ref points to a tag
- Commit hash: matches `/^[0-9a-f]{40}$/` or abbreviated — must be rejected with: "commit hashes are not valid requested refs; use a branch or tag name"

---

## 5. `AgentConfig` — Removing `repo`/`rev` Fields

**Decision**: Remove `repo: String` and `rev: String` from `AgentConfig`. The `run_agent` function reads `repository` and `requested_ref` from persisted state at startup.

**Current behavior**: `run_agent` calls `persist_never_run_state(&state_path, &config.repo, &config.rev)` when the state file does not exist. Under the new model, an absent state file means Uninitialized — `run_agent` must fail with a missing-initialization error instead of bootstrapping state from CLI args.

**Impact on `persist_never_run_state`**: This function is called only from `run_agent`. After removing the bootstrap path, it may no longer be called from the agent; the `init` command becomes the sole writer of `NeverRun` state.

---

## 6. `init` Command — New Module

**Decision**: Implement `init` in a new module `src/cli/init.rs`, following the pattern of `src/cli/agent.rs` and `src/cli/plan.rs`.

**`InitArgs`** fields (in `src/cli/args.rs`):
- `pub repository: String` — positional
- `pub ref_: String` — positional (named `requested_ref` internally; `ref` is a Rust keyword)
- `pub force: bool` — `--force` flag

**State written by `init`**:
- Writes `PersistedProvenanceState` with `reconciliation.status = NeverRun`, `desired_state.repository`, `desired_state.requested_ref`
- On `--force` with unchanged repo/ref: preserve existing reconciliation and deterministic state; clear detached flag only
- On `--force` with changed repo/ref: reset reconciliation to `NeverRun`; MAY clear deterministic state (retained snapshots)

---

## 7. Version Bump

**Decision**: `0.8.2` → `1.0.0`.

**Rationale**: Removing `--repo` and `--rev` from `plan`, `apply`, `agent`, and `explain` is a breaking CLI change. Per the release version policy (CLAUDE.md and constitution), any externally observable breaking change requires a major version increment. This is the first such breaking change since the project's 0.x lineage; `1.0.0` is the correct target.

**Alternatives considered**:
- `0.9.0` minor bump — rejected; the spec explicitly requires a major version increment for breaking CLI changes.

---

## 8. `explain` — Removing `--repo`/`--rev` Fallback

**Decision**: Remove `--repo` and `--rev` from `ExplainArgs`. `resolve_explain_target` in `src/cli/explain.rs` must be simplified to read exclusively from persisted state; the "fallback to persisted state" logic becomes the only path.

**Rationale**: The spec states `explain` inspects currently active entities derived from initialized tracking configuration. Per-invocation override would allow inspecting hypothetical configurations not matching active state, which is inconsistent with the command's purpose.

---

## 9. Verification Scenario Runner and Existing Scenarios

**Problem**: `render_coreops_action` in `src/core/verification_model.rs` currently hard-codes `--repo <path> --rev <ref>` into the constructed command for `Apply`, `Plan`, and `Explain` actions (lines 866–903). After removing those flags from the CLI, every existing scenario generates a broken command string.

Additionally, all 9 existing accepted scenarios specify `repository_source` and `revision` directly on `coreops_action` steps — a pattern that will no longer be valid for `apply`/`plan`/`explain`.

**Decision**: Add `Init` to `VerificationCoreOpsActionKind`; update `render_coreops_action` to:
- `Init`: emit `core-ops init <repo> <ref> [--force]` using `repository_source` and `revision`
- `Apply`, `Plan`, `Explain`: remove `--repo`/`--rev`; do not read `repository_source`/`revision` from action
- `Agent`: same (was already optional; remove the optional path too)

Make `repository_source` and `revision` on `VerificationCoreOpsAction` `#[serde(default)]` (they are only meaningful for `init` steps after this change).

**Existing scenario updates required** (all 9 scenarios that use `action: apply`/`action: plan`/`action: agent`):

| Scenario | Change |
|---|---|
| `minimal-accepted.yaml` | Add `init` step before first apply |
| `minimal-candidate.yaml` | Add `init` step before apply |
| `accepted-layered-convergence-idempotency.yaml` | Add `init` step before first apply |
| `accepted-layered-upgrade-transition.yaml` | Add `init` before `apply-baseline`; add `init --force` before `apply-upgrade` |
| `accepted-mount-convergence-reboot.yaml` | Add `init` before first apply |
| `accepted-mount-removal-ordering.yaml` | Add `init` before `apply-v1`; add `init --force` before `apply-v2` |
| `accepted-mount-blocked-failure.yaml` | Add `init` before apply |
| `accepted-partial-apply-verification-failure.yaml` | Add `init` before apply |
| `accepted-config-change-restart.yaml` | Add `init` before `apply-v1`; add `init --force` before `apply-v2` |

For multi-revision scenarios (upgrade-transition, mount-removal-ordering, config-change-restart): the second `init --force <repo> <new-ref>` changes the tracked ref so the subsequent `apply` targets the new desired state. The `revision` field in these scenarios is already a tag name (e.g. `demo-uat-v3`) resolvable in the fixture repo — no fixture repo changes are needed.

**Rationale**: The scenario runner is not a visible public contract, but it generates the commands executed on the VM. If it produces `--repo`/`--rev` for `apply`, the scenarios will fail on the first step after the CLI change. This is a **required** update, not optional.

---

## 10. `parse_persisted_state_text` — Behavior After Change

**Current**: Returns `Option<PersistedProvenanceState>` — `None` on parse failure.

**After change**: `read_persisted_state` wraps the `None` case with `Err(StateError::Corrupt(path.display().to_string()))`. `parse_persisted_state_text` signature stays the same (used by `status` for its explicit absent-vs-corrupt check).

**Status command special case**: `status` currently matches on `read_persisted_state` results. After the change, it must add an `Err(StateError::Corrupt(...))` arm and report it clearly — "state file at {path} is corrupt; run `core-ops init <repository> <ref> --force` to recover".
