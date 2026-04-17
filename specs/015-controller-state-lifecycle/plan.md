# Implementation Plan: Controller State Model and Lifecycle

**Branch**: `015-controller-state-lifecycle` | **Date**: 2026-04-14 | **Spec**: [spec.md](spec.md)  
**Input**: Feature specification from `specs/015-controller-state-lifecycle/spec.md`

## Summary

Introduce a first-class `core-ops init` command that persists `repository` and `requested_ref` in controller state, remove `--repo`/`--rev` from `plan`, `apply`, `agent`, and `explain` (all source exclusively from persisted configuration), add a `detached` lifecycle state entered after snapshot rollback, and surface corrupt state files as explicit errors distinct from absent state. This is a breaking CLI change requiring a major version bump: `0.8.2` → `1.0.0`.

## Technical Context

**Language/Version**: Rust 2021  
**Primary Dependencies**: clap 4, serde / serde_json, miette, thiserror, tempfile  
**Storage**: JSON state file at `/var/lib/core-ops/status.json` (atomic write via tempfile)  
**Testing**: `cargo test` and `cargo clippy --all-targets -- -D warnings`  
**Target Platform**: Linux (Fedora CoreOS / systemd hosts)  
**Project Type**: CLI binary (`core-ops`)  
**Performance Goals**: No new I/O hot paths; init is a one-time operator action  
**Constraints**: Backward-compatible state schema (new fields use `#[serde(default)]`); no breaking change to on-disk state format  
**Scale/Scope**: Single-host controller; ~10 source files affected

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] Functional core and imperative shell boundaries are explicit; side effects are isolated. — `init` logic is pure (validation, state construction); I/O is isolated in `src/io/state.rs`.
- [x] Desired/observed state, reconciliation plans, and outcomes are represented as data. — Lifecycle state is derived from `PersistedProvenanceState` fields; the `detached` flag is explicit data.
- [x] Abstractions are minimal and justified; complexity tracking added if needed. — One new field, one new error variant, one new CLI module; no new abstraction layers.
- [x] Effects, assumptions, and failure modes are explicit in interfaces and returns. — `StateError::Corrupt` makes the corrupt-file case typed and named; all commands fail explicitly on absent/corrupt state.
- [x] Idempotence and convergence strategy are defined, including retry behavior. — `init` without `--force` fails when already initialized; `init --force` with unchanged repo/ref preserves history. No retry behavior needed.
- [x] Open standards and native interfaces are preferred; deviations justified. — Git ref resolution uses existing repo infrastructure; no new dependencies.
- [x] Observability plan covers diffs, plans, actions, failures, and dry-run/audit needs. — `status` exposes lifecycle state and detached revision; `plan` annotates detached mode; agent emits detached message.
- [x] Provenance and status surfaces identify reconciler revision, desired-state revision, and applied outcome in machine-readable form. — `desired_state.repository`, `desired_state.requested_ref`, `reconciliation.last_applied_revision` exposed via `status`.
- [x] Safe defaults are documented; destructive actions require explicit intent. — `init --force` is the explicit recovery/override path; absent `--force` fails safely.
- [x] Compatibility impact is assessed; breaking changes are documented with migration. — `--repo`/`--rev` removal is a breaking CLI change; operator migration documented in quickstart.
- [x] Release version policy impact is assessed. — Major bump required; `0.8.2` → `1.0.0`.
- [x] Release intent is explicitly classified. — **`major`** — breaking CLI change (argument removal from four commands).
- [x] Changelog impact is assessed. — `CHANGELOG.md` must be updated; release-intent artifact required.
- [x] Rust changes include the required validation gate plan. — `cargo test` + `cargo clippy --all-targets -- -D warnings`.
- [x] Test strategy covers invariants, external behavior, convergence, and failures. — See test strategy below.
- [x] If the change affects externally visible host behavior, a VM-backed scenario is planned or an explicit exemption is recorded. — **New behaviors exempted**: `init`, lifecycle state derivation, `detached` flag, and corrupt-state error path do not mutate systemd/Quadlet/filesystem state. **Existing scenarios**: removing `--repo`/`--rev` breaks the scenario runner (`render_coreops_action` emits those flags); all 9 existing accepted scenarios are updated to add `init` steps and the scenario runner gains an `Init` action kind (step 11–12 in the implementation sequence).
- [x] Modules are structured to be regenerable from specs and tests.

## Project Structure

### Documentation (this feature)

```text
specs/015-controller-state-lifecycle/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── cli-init.md
│   ├── cli-command-changes.md
│   └── error-messages.md
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code

```text
src/
├── core/
│   ├── errors.rs             # Add StateError::Corrupt
│   ├── types.rs              # Add detached: bool to PersistedProvenanceState
│   └── verification_model.rs # Add Init to VerificationCoreOpsActionKind; update render_coreops_action
├── cli/
│   ├── args.rs               # Add InitArgs + Commands::Init; remove --repo/--rev from 4 commands
│   ├── init.rs               # NEW: run_init implementation
│   ├── agent.rs              # Remove repo/rev from AgentConfig; add lifecycle guard
│   ├── plan.rs               # Source repo/ref from persisted state
│   ├── apply.rs              # Source repo/ref from persisted state; Detached mode guard
│   └── explain.rs            # Remove --repo/--rev fallback
├── io/
│   └── state.rs              # read_persisted_state: present-but-invalid → Err(Corrupt)
└── main.rs                   # Add Init dispatch; remove CORE_OPS_REPO/CORE_OPS_REV for 4 commands

tests/fixtures/verification/scenarios/
├── minimal-accepted.yaml                          # Add init step
├── minimal-candidate.yaml                         # Add init step
├── accepted-layered-convergence-idempotency.yaml  # Add init step
├── accepted-layered-upgrade-transition.yaml       # Add init + init --force steps
├── accepted-mount-convergence-reboot.yaml         # Add init step
├── accepted-mount-removal-ordering.yaml           # Add init + init --force steps
├── accepted-mount-blocked-failure.yaml            # Add init step
├── accepted-partial-apply-verification-failure.yaml # Add init step
└── accepted-config-change-restart.yaml            # Add init + init --force steps

Cargo.toml                    # version = "1.0.0"
CHANGELOG.md                  # Add unreleased entry
changes/                      # Release intent fragment (major)
```

**Structure Decision**: Single Rust project. No new crates required.

---

## Phase 0: Research

See [research.md](research.md) for full decision records. Key findings:

1. **StateError::Corrupt**: Add `Corrupt(String)` variant; update `read_persisted_state` to return it when file is present but `parse_persisted_state_text` returns `None`. Callers using `.ok().flatten()` are safe post-apply (audit-only); pre-action callers must be updated.

2. **Detached flag**: `#[serde(default)] pub detached: bool` on `PersistedProvenanceState`; existing files without the field deserialize as `false`. No schema version bump needed.

3. **`init` ref validation**: Resolve ref against repository using existing repo infrastructure; check that resolved ref starts with `refs/heads/` or `refs/tags/`; reject commit hashes with named error.

4. **`AgentConfig`**: Remove `repo`/`rev`; read from persisted state at startup; absent → fail with initialization error; corrupt → fail with corrupt error; detached → emit message and exit cleanly.

5. **Version bump**: `0.8.2` → `1.0.0`; breaking CLI change (removal of `--repo`/`--rev` from four commands).

---

## Phase 1: Design

See [data-model.md](data-model.md), [contracts/](contracts/), and [quickstart.md](quickstart.md).

### Data model changes

- `StateError::Corrupt(String)` — new variant in `src/core/errors.rs`
- `PersistedProvenanceState.detached: bool` — new field with `#[serde(default)]`
- `AgentConfig.{repo,rev}` — removed
- `{Plan,Apply,Agent,Explain}Args.{repo,rev}` — removed
- `InitArgs` — new struct (positional `repository`, positional `requested_ref`, `--force` flag)

### Interface contracts

- `contracts/cli-init.md` — full `core-ops init` contract including error messages and reinitialization rules
- `contracts/cli-command-changes.md` — argument removals, Detached mode behavior per command
- `contracts/error-messages.md` — canonical error message strings for all lifecycle states

### Implementation sequence

1. `StateError::Corrupt` + `read_persisted_state` fix
2. `detached` flag on `PersistedProvenanceState`
3. `InitArgs` + `Commands::Init` in args.rs; remove `--repo`/`--rev` from four commands
4. Implement `src/cli/init.rs`
5. Update `src/cli/agent.rs` (remove `repo`/`rev`; add lifecycle guard)
6. Update `src/cli/plan.rs`
7. Update `src/cli/apply.rs`
8. Update `src/cli/explain.rs`
9. Update `src/cli/status.rs`
10. Update `src/main.rs`
11. Update `src/core/verification_model.rs`: add `Init` to `VerificationCoreOpsActionKind`; update `render_coreops_action` to emit `core-ops init` for `Init`, remove `--repo`/`--rev` from `Apply`/`Plan`/`Explain`/`Agent` cases; make `repository_source`/`revision` `#[serde(default)]` on `VerificationCoreOpsAction`
12. Update all 9 existing accepted scenarios: add `init` step before each first `apply`; add `init --force` before each subsequent apply that changes the tracked ref (upgrade-transition, mount-removal-ordering, config-change-restart)
13. Version bump + CHANGELOG + release-intent fragment

### Test strategy

- Unit: `StateError::Corrupt` returned for present-but-invalid file; `Ok(None)` for absent; lifecycle state derivation from `PersistedProvenanceState`
- Unit: `init` ref validation rejects bare commit hashes; accepts branch and tag names
- Unit: `init --force` with unchanged repo/ref preserves reconciliation history and clears detached flag
- Unit: `init --force` with changed repo/ref resets to NeverRun
- Unit: `run_agent` fails with named error on absent state; fails with named error on corrupt state; exits cleanly on detached state
- Integration: `core-ops init` followed by `core-ops plan` uses persisted repo/ref
- Integration: `core-ops plan`/`apply`/`agent` without prior init produce actionable error
- Validation gate: `cargo test` + `cargo clippy --all-targets -- -D warnings`
- VM-backed scenario (new behaviors): **exempted** — `init`, lifecycle state derivation, `detached` flag, and corrupt-state error path do not mutate systemd/Quadlet/filesystem state on the host
- VM-backed scenario (existing scenarios): all 9 existing accepted scenarios are updated to add `init` (and `init --force` where ref changes); these remain accepted after the update

### Post-design Constitution re-check

All items from the initial check remain satisfied. One gap found and corrected during review:

- **Gap corrected**: The initial exemption on VM-backed scenarios overlooked that removing `--repo`/`--rev` breaks `render_coreops_action` in the scenario runner and all 9 existing accepted scenarios. Steps 11–12 in the implementation sequence address this. The exemption now correctly covers only the new behaviors (init, detached, corrupt state), not the scenario updates.
- The `detached` flag and `StateError::Corrupt` are explicit data and typed errors (Principles 2 and 4)
- `init` is the only writer of NeverRun state; all other writes are lifecycle-driven (Principle 8)
- Breaking CLI change documented with migration path (Principle 9)
- Release governance artifacts identified: `Cargo.toml` → `1.0.0`, `CHANGELOG.md` entry, `changes/` fragment (Principle 13)
