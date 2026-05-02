# Implementation Plan: Source Repository Layout Formalization

**Branch**: `016-source-repository-layout` | **Date**: 2026-05-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/016-source-repository-layout/spec.md`

## Summary

Formalize the on-disk shape of CoreOps source repositories around payload-kind directories (`quadlet/`, `systemd/`, `config/`), an optional `service.yaml` declaring `config-root` for variant services, and a host overlay tree that mirrors the service shape directly (no `overrides/` segment). Drop-ins keep systemd-native filesystem parity (`<unit>.<ext>.d/<file>.conf`). The legacy `quadlets/` and `quadlet-overrides/` parsers are removed in a single major-version cut. Operators install a vendor-neutral agent skill bundle via a new `core-ops skill install` subcommand that targets `.agents/skills/core-ops-source-repo/` per agentskills.io. Reference example repositories ship in-tree alongside the spec.

## Technical Context

**Language/Version**: Rust 2021 (stable toolchain), as established by the existing `core-ops` crate at v1.0.0; this feature is the trigger for the v2.0.0 major bump.
**Primary Dependencies**: `clap` 4.5 (derive), `serde` 1.0 (derive), `serde_yaml` 0.9, `serde_json` 1.0, `miette` 7.2 (fancy diagnostics), `thiserror` 1.0, `tempfile` 3.10. No new runtime dependencies are required by this feature.
**Storage**: Source repository on filesystem (input); existing canonical status snapshot at `/var/lib/core-ops/status.json` (output). The status snapshot gains a `layout-version: "1"` field to record which layout produced it.
**Testing**: `cargo test` with integration tests under `tests/integration/`, fixtures under `tests/fixtures/`, plus the in-tree reference examples under `specs/016-source-repository-layout/examples/` exercised as integration fixtures. `cargo clippy --all-targets -- -D warnings` gate is mandatory per the constitution.
**Target Platform**: Linux host running systemd ≥ 252 with Podman ≥ 4.4 (Quadlet support). Source repository authoring is platform-agnostic; the binary runs on any Linux host capable of executing `core-ops` today.
**Project Type**: Single-project CLI/library (Rust workspace with three binaries: `core-ops`, `core-ops-verify`, `core-ops-release`).
**Performance Goals**: Layout parse for a typical source repository (≤ 50 services, ≤ 10 hosts) completes in under 100 ms cold and under 20 ms warm. The new `Collection.get`-equivalent path inside the loader is a single deterministic filesystem walk; no quadratic or filter-chain behavior is introduced.
**Constraints**: Deterministic load order (lex-sorted by service id, then file path within payload kind, then drop-in filename). All diagnostics MUST carry a `miette` source span pointing at the offending file or YAML key. The migration of the live legacy repository is mechanical — no semantic re-interpretation is permitted.
**Scale/Scope**: Today: one source repository (`~/code/ulthar/repo/`) with four services, one host. Designed for: source repositories with up to ~50 services and ~10 hosts (10× headroom). Skill bundle is a fixed-size artifact (one `SKILL.md` plus optional small assets, target ≤ 10 KB compressed).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **Functional core / imperative shell**: Layout parsing is pure (`source-repo path → DesiredState | LayoutError`). All filesystem mutation remains in the apply path. ✅
- **Declarative state model**: `DesiredState` is the canonical data structure consumed by the planner; updates in this feature add fields, do not introduce hidden behavior. ✅
- **Simplicity over cleverness**: Removes two parsers (legacy `quadlets/`, `quadlet-overrides/`), introduces one optional manifest with one initial key. Net surface decrease. ✅
- **Explicit effects / explicit failure**: Every validation rule (FR-009 to FR-013, FR-016 to FR-018) returns a typed `LayoutError` with source span. No silent recovery. ✅
- **Idempotence and convergence**: Re-loading an unchanged source repository yields a byte-identical `DesiredState` (FR-015, SC-005). Re-applying produces no host actions. ✅
- **Open standards / native interfaces**: `<unit>.<ext>.d/<file>.conf` mirrors `/etc/systemd/system/<unit>.<ext>.d/`. Skill bundle targets agentskills.io's `.agents/skills/`. No proprietary formats. ✅
- **Observability**: Plan output reports source-repo path, host id, resolved service set, content hash per service tree, and the new `layout-version` field. Diagnostics carry source spans. ✅
- **Provenance / status**: `status.json` gains `layout-version: "1"`. Future revisions can detect which layout produced a snapshot, supporting upgrade reasoning. ✅
- **Safe defaults / explicit power**: Hard cut on legacy is a major bump and a load-time fatal error, never a silent reinterpretation. The skill subcommand requires explicit invocation (no auto-install). ✅
- **Conservative public evolution**: This change *is* a non-conservative evolution — a major bump. The justification is enumerated in the spec (Q1=a, sole user, mechanical migration). The release-intent artifact will declare `major`. ✅
- **Release governance**: `Cargo.toml` will move from 1.0.0 → 2.0.0; `CHANGELOG.md` `[Unreleased]` block will enumerate every externally visible change; `changes/016-source-repository-layout.md` will declare `release_intent: major`. ✅
- **Rust validation gates**: Plan includes `cargo test` and `cargo clippy --all-targets -- -D warnings` for every milestone in Phase B. No exemption. ✅
- **Test contract**: Tests target invariants (load determinism, dispatch correctness, validation rules), externally visible behavior (CLI surface for `skill install`), convergence (idempotency), and failure modes (every diagnostic class). VM-backed scenario coverage is required for the apply-path destination changes; this is recorded in the plan and slated for Phase B. ✅
- **VM-backed scenario**: A new scenario under `tests/fixtures/verification/scenarios/` will exercise a host that selects a service with `config-root` differing from the service id, and assert the deployed `/etc/<config-root>/` paths. Recorded as a Phase B deliverable. ✅
- **Regenerability**: The reference examples (FR-023) plus the spec and contracts permit a from-scratch reimplementation of the parser. `data-model.md` and `contracts/` make struct shapes and YAML schemas explicit. ✅

All gates pass. No `Complexity Tracking` entries required.

## Project Structure

### Documentation (this feature)

```text
specs/016-source-repository-layout/
├── plan.md                              # This file (/speckit.plan output)
├── spec.md                              # Feature specification (already authored)
├── research.md                          # Phase 0 output
├── data-model.md                        # Phase 1 output
├── quickstart.md                        # Phase 1 output
├── contracts/                           # Phase 1 output
│   ├── service-yaml.schema.yaml         # service.yaml JSON Schema
│   ├── host-yaml.schema.yaml            # host.yaml JSON Schema
│   ├── layout.md                        # Normative on-disk layout contract
│   └── skill-cli.md                     # `core-ops skill install` CLI contract
├── checklists/
│   └── requirements.md                  # Spec quality checklist (already authored)
├── examples/                            # FR-023 reference repositories (Phase C)
│   ├── 01-minimal-single-service/
│   ├── 02-variant-config-root/
│   ├── 03-multi-unit-with-dropins/
│   └── 04-host-overlay/
├── skill/                               # Phase D source for the skill bundle
│   ├── SKILL.md                         # The skill itself
│   └── assets/                          # Any supporting assets (currently none planned)
└── tasks.md                             # Phase 2 output (/speckit.tasks - not by this command)
```

### Source Code (repository root)

```text
core-ops/
├── Cargo.toml                           # Bumped 1.0.0 → 2.0.0 by this feature
├── CHANGELOG.md                         # [Unreleased] block populated
├── changes/
│   └── 016-source-repository-layout.md  # Release intent: major
├── src/
│   ├── cli/
│   │   ├── args.rs                      # +Skill subcommand variant
│   │   ├── skill.rs                     # NEW — `core-ops skill install` impl
│   │   └── mod.rs                       # +pub mod skill
│   ├── core/
│   │   └── types.rs                     # DesiredState gains layout-version, ServiceDefinition gains config_root
│   └── io/
│       └── repo.rs                      # MAJOR REWRITE — payload-kind dispatch, service.yaml, host overlay shape, legacy rejection
├── tests/
│   ├── integration/
│   │   ├── test_source_repo_layout.rs   # NEW — load + diagnostic coverage
│   │   ├── test_skill_install.rs        # NEW — CLI contract for skill install
│   │   ├── test_config_roots.rs         # UPDATED — exercises config-root semantics post-rewrite
│   │   └── test_deterministic_planning.rs # UPDATED — assertions hold against new layout
│   └── fixtures/
│       ├── source_repos/                # NEW — fixtures aligned to formalized layout
│       │   ├── minimal/
│       │   ├── variant/
│       │   ├── multi_unit/
│       │   └── host_overlay/
│       ├── verification/
│       │   └── scenarios/
│       │       └── source-repo-variant-config-root.yaml  # NEW VM scenario
│       └── layered_overrides/           # REMOVED — legacy fixture
└── scripts/
    └── migrate-legacy-source-repo.sh    # NEW — one-off mechanical migration for ~/code/ulthar/repo/
```

**Structure Decision**: Single Rust project (existing). Feature changes are concentrated in `src/io/repo.rs` (rewrite), `src/cli/skill.rs` (new), and a small set of integration tests + new in-tree examples. No workspace restructuring.

## Complexity Tracking

> No constitution violations. Section intentionally empty.

## Phasing

This feature ships in four phases inside the same branch. Each phase produces a reviewable artifact set; phases B–D are gated by green `cargo test` and `cargo clippy --all-targets -- -D warnings`.

- **Phase A — Specification (this commit)**: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`. No code change.
- **Phase B — Parser & CLI rewrite**: rewrite `src/io/repo.rs` to the formalized layout; remove legacy parsing; implement `src/cli/skill.rs`; add new integration tests and the VM scenario fixture; bump `Cargo.toml` 1.0.0 → 2.0.0; add `changes/016-source-repository-layout.md` with `release_intent: major`; populate `CHANGELOG.md` `[Unreleased]`.
- **Phase C — Reference examples**: author the four example repositories under `specs/016-source-repository-layout/examples/`; wire them as integration fixtures; add the migration script `scripts/migrate-legacy-source-repo.sh` and document its usage in `quickstart.md`.
- **Phase D — Skill bundle**: author `specs/016-source-repository-layout/skill/SKILL.md` from the spec, examples, and contracts; wire it into `core-ops skill install` so the binary embeds the bundle at compile time (e.g. via `include_str!` / `include_dir!` of the skill subdir); add round-trip tests asserting byte identity across `default`, `--global`, and `--print` modes.

`tasks.md` (Phase 2 of `/speckit.*`) decomposes B–D into ordered work items.
