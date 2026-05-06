# Implementation Plan: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Branch**: `017-real-world-validation` | **Date**: 2026-05-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/017-real-world-validation/spec.md`

## Summary

Spec/017 closes the long-open follow-up at `docs/follow-ups.md` lines 87–99 (rich real-life examples + QnA-of-known-limitations) by publishing five real-world homelab examples under top-level `examples/`, and resolves a v2.0.0 regression where the four spec/016 example READMEs reference a `--source-repo` flag that has never been implemented. Adds the missing flag to `core-ops plan`, `apply`, and `explain` for stateless invocation against a filesystem path (no `core-ops init` required), with git-aware provenance recording (clean-checkout SHA / `(stateless+dirty)` / `(stateless)` sentinels). Removes the four superseded spec/016 example fixtures. Cleans up stale CLI documentation.

Approach: reuse existing parser/planner/applier; add a CLI flag plumbed through `src/cli/{plan,apply,explain}.rs` that bypasses init'd-state lookup and resolves desired-state directly from the path. Reuse the existing shell-to-git pattern (already used in `src/io/repo.rs`, `src/cli/init.rs`) for ref detection. No new runtime dependencies. Per-example integration tests assert parser load + flag invocation; per-mode unit tests assert CLI argument parsing and provenance shape.

## Technical Context

**Language/Version**: Rust 2021 (existing toolchain)
**Primary Dependencies**: clap 4.5 (derive), serde 1.0, serde_yaml 0.9, serde_json 1.0, miette 7.2 (fancy diagnostics), thiserror 1.0, tempfile 3.10. **No new runtime dependencies.** Git invocation via `std::process::Command::new("git")` following the established pattern at `src/cli/init.rs:52`, `src/io/repo.rs:1312/1343/1372`, `src/io/release_governance.rs:367/440`, `src/cli/verification.rs:2068/2086/2090/2103`.
**Storage**: Existing `/var/lib/core-ops/status.json` for init'd mode (unchanged). Stateless plan writes nothing under `/var/lib/`; stateless apply writes audit + status with path-based provenance (see FR-013); stateless explain writes nothing. Operator-explicit `--audit-dir` honored across both modes (see FR-012 plus 2026-05-05 clarification).
**Testing**: `cargo test` and `cargo clippy --all-targets -- -D warnings` (project standard). New surfaces: per-example integration tests at `tests/integration/test_examples_<NN>_<slug>.rs` (5 files), per-mode integration tests at `tests/integration/test_stateless_{plan,apply,explain}.rs` (3 files). Existing helper at `tests/integration/source_repo_support.rs:20` (`EXAMPLES_DIR = "specs/016-source-repository-layout/examples"`) is updated or replaced when the four spec/016 example dirs are removed.
**Target Platform**: Linux (Fedora CoreOS canonical, other systemd-based hosts expected to work). Stateless mode functions on any platform the existing parser supports.
**Project Type**: Rust CLI tool (single project, Option 1 in template).
**Performance Goals**: Per-example parse + plan latency ≤ 100ms each (existing parser overhead bounds). Aggregate added test-suite cost ≤ 500ms (8 new tests × ~50ms each).
**Constraints**: No new runtime dependencies; reuse existing git-CLI pattern. Stateless apply MUST NOT mutate init'd `desired_state.repository` / `desired_state.requested_ref` (FR-013, SC-009). All hostnames in `examples/` use RFC 2606 reserved domains; all IP literals use RFC 5737 documentation ranges. No file under `examples/` may reference operator-private values from `~/code/ulthar/repo/`.
**Scale/Scope**: 5 example directories (~30–60 files each, ~250 files total), 3 CLI commands modified, 1 module possibly added (`src/io/source_ref.rs` for git ref detection — TBD during implementation), 8 new tests, 4 spec/016 example dirs deleted, 4 stale-doc surfaces updated.

## Constitution Check

*GATE: passed before Phase 0 research. Re-evaluation after Phase 1 design records the same outcome — design did not introduce new violations.*

- **Functional core / imperative shell** ✓ — Stateless mode adds a new entry-point boundary (path → in-memory `EvaluationInput`) but reuses the existing parser, planner, applier. No new core or side-effect surfaces.
- **Declarative state** ✓ — `EvaluationInput` representation unchanged; only the source-of-truth resolution differs.
- **Simplicity over cleverness** ✓ — One additional flag plus a code path that bypasses init'd-state lookup. No new abstractions; sentinel ref strings fit the existing `String` shape of `DesiredStateProvenance.requested_ref` (verified at `src/core/types.rs:555`).
- **Explicit effects / failures** ✓ — FR-014 (path validation), FR-015 (non-git supported), FR-013 (provenance shape) all explicit. Failure modes in Edge Cases.
- **Idempotence & convergence** ✓ — Stateless apply has identical convergence semantics to init'd apply against the same tree.
- **Open standards / native interfaces** ✓ — git, systemd, Quadlet preserved. Filesystem path-based source is the most native of all options.
- **Observability** ✓ — Plan output unchanged. Audit chain extends to stateless via path-based provenance. Sentinels (`(stateless)` / `(stateless+dirty)` / SHA) make the source mode visible in `core-ops status`.
- **Provenance & traceability** ✓ — FR-013 + 2026-05-05 clarification (Q3) record the most-precise traceable revision available: clean-checkout SHA when discoverable, dirty sentinel when working tree differs, non-git sentinel otherwise.
- **Safe defaults** ✓ — Default mode (no `--source-repo`) remains init'd. Stateless requires explicit flag.
- **Compatibility** ✓ — Existing init'd CLI surface unchanged; stateless flag is purely additive within each command.
- **Release version policy** — Adds CLI surface (`--source-repo` × 3 commands, FR-016 help text) ⇒ `minor` per spec/011 inferred-bump rules. Removes `specs/016-source-repository-layout/examples/` (4 directories). Whether the validator infers `major` from spec/example deletions is governed by `src/core/release_governance.rs` rules and `core-ops-release validate` is authoritative. Declared `release_intent: minor`; bump to `major` (and version `3.0.0`) if validator demands.
- **Release intent artifact** — `changes/017-real-world-validation.md` declares `release_intent: minor` (or `major` per validator).
- **Changelog discipline** — Re-render via `cargo run --bin core-ops-release -- changelog --write`; post-merge `core-ops-release promote` (shipped v2.1.0) handles the `[Unreleased]` → `[<version>]` transition.
- **Rust validation gate** ✓ — `cargo test` and `cargo clippy --all-targets -- -D warnings` are required before merge. No exemption.
- **Test strategy** ✓ — Invariants: parser load succeeds for each example; init'd `desired_state.*` unchanged after stateless apply (SC-009). External behavior: `--source-repo` flag exit codes / output. Convergence: stateless apply idempotence covered. Failure: non-directory path, missing `--host`.
- **VM-backed scenario** — **Exemption recorded.** Stateless mode introduces a CLI entry-point variation but no new mutation classes — the actual host-state changes performed by stateless apply are identical to those performed by init'd apply against the same tree. Existing apply VM scenarios at `tests/fixtures/verification/scenarios/` remain authoritative for mutation semantics. Stateless mode is exercised at the unit + integration test layer. Exemption justification: spec.md Constitution Alignment section + the rationale here. Per Principle 10's exemption clause.
- **Regenerability** ✓ — Examples derivative of public upstream sources cited per-example. Synthesis table re-derivable from translation artifacts.

**Constitution Check: PASS** with one explicit, narrow, machine-checkable VM-scenario exemption.

## Project Structure

### Documentation (this feature)

```text
specs/017-real-world-validation/
├── plan.md              # This file
├── research.md          # Phase 0 output (technical decision log)
├── data-model.md        # Phase 1 output (provenance string conventions + synthesis-table schema)
├── quickstart.md        # Phase 1 output (operator walkthrough)
├── contracts/           # Phase 1 output
│   ├── cli-flag.md      # `--source-repo` CLI contract
│   └── synthesis-table.md  # friction-classification table contract
├── checklists/
│   └── requirements.md  # written by /speckit.specify
├── spec.md              # written by /speckit.specify
└── tasks.md             # NEXT — produced by /speckit.tasks
```

### Source Code (repository root)

```text
core-ops/
├── examples/                                   # NEW — 5 real-world examples
│   ├── 01-caddy-whoami/
│   │   ├── README.md
│   │   ├── services/<svc>/{service.yaml?,quadlet/,systemd/?,config/?}
│   │   └── hosts/<example-host>/{host.yaml,<svc>/{quadlet/,systemd/?,config/?}}
│   ├── 02-nextcloud/                           # multi-Container, intra-service network, persistent storage, TLS
│   ├── 03-immich/                              # GPU device, multi-network, ML worker
│   ├── 04-traefik-authelia/                    # ForwardAuth composition, cross-service network
│   └── 05-observability/                       # Prometheus + Grafana + node-exporter + cadvisor; host-scope sidecars
├── src/
│   ├── cli/
│   │   ├── args.rs                             # MOD — add `source_repo: Option<PathBuf>` to PlanArgs/ApplyArgs/ExplainArgs
│   │   ├── plan.rs                             # MOD — branch on source_repo, build EvaluationInput from path
│   │   ├── apply.rs                            # MOD — same; preserve init'd desired_state.* per FR-013
│   │   └── explain.rs                          # MOD — same; pure-read
│   ├── core/
│   │   └── types.rs                            # NO CODE CHANGE — DesiredStateProvenance.requested_ref is already String; sentinels fit. (Documentation in data-model.md.)
│   └── io/
│       ├── source_ref.rs                       # NEW (or merged into existing repo.rs/init.rs) — `detect_provenance(path) -> { repository: AbsPath, requested_ref: String }`
│       └── repo.rs                             # NO CODE CHANGE expected
├── tests/integration/
│   ├── test_examples_01_caddy_whoami.rs        # NEW
│   ├── test_examples_02_nextcloud.rs           # NEW
│   ├── test_examples_03_immich.rs              # NEW
│   ├── test_examples_04_traefik_authelia.rs    # NEW
│   ├── test_examples_05_observability.rs       # NEW
│   ├── test_stateless_plan.rs                  # NEW — argument parsing, --host requirement, provenance shape, --audit-dir honored
│   ├── test_stateless_apply.rs                 # NEW — successful apply, init'd desired_state.* unchanged (SC-009), non-git path
│   ├── test_stateless_explain.rs               # NEW — read-only invocation, no /var/lib writes
│   ├── source_repo_support.rs                  # MOD — EXAMPLES_DIR repointed at top-level `examples/`, or helper deleted if redundant
│   └── mod.rs                                  # MOD — register the 8 new modules
├── docs/
│   ├── follow-ups.md                           # MOD — remove the now-shipped Init Command paragraphs; preserve still-valid bullets
│   └── development.md                          # MOD — line 228 example updated (use --source-repo or init+plan)
├── infra/repo/
│   └── README.md                               # MOD — lines 32, 35, 38 updated
├── specs/016-source-repository-layout/
│   ├── examples/                               # DELETED (4 subdirs removed)
│   ├── spec.md                                 # MOD — FR-023 carries supersession note pointing at top-level examples/
│   └── tasks.md                                # MOD — T101–T104 annotated `[X] [SUPERSEDED by spec/017]`
├── README.md                                   # MOD — add `## Real-World Examples` section
├── Cargo.toml                                  # MOD — version 2.1.1 → 2.2.0 (or 3.0.0 if validator demands)
├── CHANGELOG.md                                # MOD — re-rendered via core-ops-release changelog --write
└── changes/
    └── 017-real-world-validation.md            # NEW — release fragment, release_intent: minor (or major)
```

**Structure Decision**: Single Rust project (template Option 1). Existing layout preserved; new artifacts under `examples/`, `tests/integration/test_examples_*.rs`, `tests/integration/test_stateless_*.rs`, plus per-spec docs under `specs/017-real-world-validation/`.

## Complexity Tracking

> Filled only if Constitution Check has violations to justify.

No violations to justify. The single exemption (no new VM-backed scenario) is narrow, explicit, and recorded above; existing apply VM scenarios remain authoritative for the mutation classes that stateless apply exercises.
