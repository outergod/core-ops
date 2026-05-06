# Research: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Phase**: 0 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

This document records the technical decisions for spec/017's implementation phase. The five-setup roster, synthesis-table classification semantics, and stateless-mode user-visible behavior are spec-level decisions already locked in `spec.md` (including the 2026-05-05 clarifications section). This document covers the *implementation* decisions: how to detect git provenance, how to wire the new flag, how to handle the existing helper that loads spec/016 examples, and which public upstream sources back each setup's translation.

---

## D1 — Git ref detection strategy for stateless mode

**Decision**: Shell out to the system `git` binary via `std::process::Command::new("git")` for git presence and ref detection. Reuse the established pattern already present in `src/cli/init.rs:52`, `src/io/repo.rs:1312/1343/1372`, `src/io/release_governance.rs:367/440`, and `src/cli/verification.rs:2068`+.

**Rationale**:
- No new runtime dependency (Cargo.lock currently has no `git2` entry).
- Consistent with how every other git interaction in the codebase already works.
- Sufficient for the three states the spec requires: clean checkout, dirty working tree, non-git directory.
- The git CLI is already an implicit runtime requirement of CoreOps in init'd mode.

**Alternatives considered**:
- **`git2` crate (libgit2 bindings)**: Rejected. Adds a non-trivial dependency for marginal gain. We need at most: detect `.git`, run `rev-parse HEAD`, run `status --porcelain --` against a path. CLI invocation handles all three trivially.
- **Pure Rust git parsing**: Rejected as gratuitously complex. Reading `.git/HEAD` and walking refs ourselves trades external-process cost for parsing complexity that has to track git internals.

**Algorithm** (to be implemented in `src/io/source_ref.rs` or merged into `repo.rs`):

```text
detect_provenance(path: &Path) -> { repository: AbsPath, requested_ref: String }:
  1. repository = canonicalize(path).
  2. Check `git -C path rev-parse --is-inside-work-tree` exits 0 and prints "true".
     - If not: return { repository, requested_ref: "(stateless)" }.
  3. Check `git -C path status --porcelain -- .` for non-empty output.
     - Non-empty (any modified, added, deleted, or untracked file under path):
       return { repository, requested_ref: "(stateless+dirty)" }.
  4. Capture `git -C path rev-parse HEAD` SHA (40-char hex).
     - Return { repository, requested_ref: <SHA> }.
  5. On any subprocess error: log via miette diagnostic, fall back to "(stateless)".
```

**Edge cases the implementation must handle**:
- Path is a subdirectory of a git repository — `git -C path` correctly resolves the enclosing `.git/`. The status check is scoped to `-- .` so changes in sibling directories don't pollute the cleanliness check for our path.
- Detached HEAD — `rev-parse HEAD` returns the SHA, no special handling needed.
- Shallow clone — SHA is still meaningful, no special handling.
- Submodules under the path — out of scope for v1; submodule changes may incorrectly mark "clean" since `status --porcelain --` doesn't recurse into submodule working trees by default. Document as a known limitation if encountered during translation.

**Test surface**: unit-level coverage for each branch using `tempfile`-created git repos; an integration test that exercises a real example under each state (clean / dirty / non-git).

---

## D2 — `DesiredStateProvenance` schema compatibility

**Decision**: No struct changes required. The existing `DesiredStateProvenance.requested_ref: String` (`src/core/types.rs:555`) accepts the sentinel strings `(stateless)` and `(stateless+dirty)` alongside SHA values without any schema modification.

**Rationale**:
- `requested_ref` is `String` (not a constrained enum); it already holds branch names, tag names, and SHAs without further validation.
- Adding sentinel strings is a *value-level* convention, not a *type-level* change. No serde annotations need updating.
- Test fixtures that match against `requested_ref` need to be reviewed for hardcoded values — they likely use commit-SHA-shaped strings or branch names, which the sentinels do not collide with (sentinels start with `(`, which is not valid in a git ref name per `git check-ref-format`).

**Verification**:
- `git check-ref-format -- "(stateless)"` exits non-zero — confirming the sentinel cannot collide with a real ref.
- The same is true for `(stateless+dirty)` — parens and `+` are reserved.

**Document the convention** in `data-model.md` so future readers and serde consumers understand sentinel semantics.

---

## D3 — Spec/016 example removal impact

**Decision**: Delete the four `specs/016-source-repository-layout/examples/{01-minimal-single-service, 02-variant-config-root, 03-multi-unit-with-dropins, 04-host-overlay}/` directories. Update `tests/integration/source_repo_support.rs:20` (the only code-level consumer) — either repoint `EXAMPLES_DIR` at top-level `examples/` (if any test still uses it) or delete the helper entirely if all consumers are repointed at the new spec/017 examples.

**Rationale**:
- One code consumer found via grep; the impact is bounded.
- Spec/016 spec.md FR-023 is amended with a supersession note; spec/016 tasks.md T101–T104 carry `[X] [SUPERSEDED by spec/017]` annotations that preserve the historical record without rewriting it.
- The four example dirs were layout-shape fixtures; their pedagogical role is now filled by the five real-world examples.

**Concrete changes**:
- `git rm -r specs/016-source-repository-layout/examples/{01-*,02-*,03-*,04-*}`.
- Edit `tests/integration/source_repo_support.rs:20` — repoint or remove the `EXAMPLES_DIR` constant; update or remove any tests that referenced the four spec/016 example slugs.
- Edit `specs/016-source-repository-layout/spec.md`: append a supersession note to FR-023.
- Edit `specs/016-source-repository-layout/tasks.md`: append `[SUPERSEDED by spec/017]` to T101–T104.

**Alternatives considered**:
- **Forwarding marker** (replace each dir with a `MOVED.md`): Rejected as low-value clutter. The spec.md supersession note + git history is sufficient for anyone looking back.
- **Keep spec/016 examples + add spec/017 examples**: Rejected. Two example sets in two locations is the kind of duplication the constitution Principle 3 calls out.

---

## D4 — Stale CLI documentation cleanup

**Decision**: Update three doc surfaces; leave historical spec quickstarts as time-capsules.

| Surface | Action |
|--------|-------|
| `docs/follow-ups.md` lines 7–14 ("Init Command" paragraphs about `--repo`/`--rev` removal) | Remove the now-shipped paragraphs. Preserve still-valid follow-ups in the same section: `quadlet-dir`/`systemd-unit-dir`/`state-file`/`audit-dir` arg persistence, `rollback-plan-only` re-homing, `--reinitialize` UX. |
| `docs/development.md` line 228 (`CORE_OPS_HOST=<host> core-ops plan --repo <repo> --rev <rev>`) | Replace with `core-ops plan --source-repo <PATH> --host <HOST>` (stateless example) plus a note about the init'd-mode workflow. |
| `infra/repo/README.md` lines 32, 35, 38 (`core-ops plan --repo file:///… --rev demo-uat-vN`) | Update each to use `--source-repo` against a checkout of the demo repo. |
| `specs/001-gitops-quadlet-controller/quickstart.md:18`, `specs/007-explainable-reconcile-interface/quickstart.md:73` | **No change.** Historical spec quickstarts are time-capsules of the spec at the time it was written. |

**Rationale**: Spec quickstarts are versioned artifacts whose purpose is to document the state of the system at the time the spec was authored. Updating them retroactively would erase historical context. Operational docs (`docs/`, `infra/`) describe current behavior and must reflect today's CLI.

---

## D5 — License hygiene for upstream-derived examples

**Decision**: Each example's Quadlet units are written from scratch as the implementer's interpretation of the upstream design intent. Upstream `compose.yml` / configuration files cited in the README as design references but not copied verbatim.

**Rationale**:
- Avoids inheriting upstream license terms (Nextcloud, Immich, Authelia, and most observability tooling are AGPL/MIT/Apache mixes; verbatim copy of YAML blocks may carry license obligations into core-ops AGPLv3+).
- The translation is the deliverable: showing how a real workload becomes a spec/016-conformant repository. Verbatim copying would defeat the validation purpose (it would be a port, not a translation).
- Upstream attribution is preserved in each example's README under a "Sources" heading.

**Public upstream sources** (URLs to be fetched and verified during the translation phase, one task per example in `/speckit.tasks`):

| Slug | Primary upstream sources |
|------|--------------------------|
| `01-caddy-whoami` | Caddy official documentation (`caddyserver.com/docs/quick-starts`); `traefik/whoami` container README. |
| `02-nextcloud` | Nextcloud's "Docker Compose with reverse proxy" community example (NOT the All-In-One container, which manages its own sub-containers via Docker socket and is incompatible with external orchestration). |
| `03-immich` | `immich-app/immich` repository's `docker/docker-compose.yml`. |
| `04-traefik-authelia` | Authelia's official Traefik integration documentation (`authelia.com/integration/proxies/traefik/`). |
| `05-observability` | Prometheus, Grafana, node-exporter, cadvisor official docker-compose examples; `prometheus/node_exporter` README for host-scope bind mounts. |

The implementer fetches these during Phase 2 (Translation tasks) and embeds canonical URLs in each example's `README.md`.

---

## D6 — Stateless mode and `--audit-dir` interaction

**Decision**: Stateless `plan` and `apply` honor an explicit `--audit-dir <DIR>` flag exactly as init'd mode does. Stateless `explain` is pure-read; it does not write audit. Pre-locked by 2026-05-05 clarification (Q4 in spec.md).

**Rationale**: documented in spec.md Clarifications. Implementation impact: the existing audit-dir handling in `src/cli/{plan,apply}.rs` requires no special-case for stateless mode — the same code path writes to the operator-specified directory regardless of source mode. The `/var/lib/` separation is enforced by *not* writing the persisted controller state, not by suppressing `--audit-dir`.

---

## D7 — Stateless mode argument validation

**Decision**: Argument validation rules for stateless invocations:
1. `--source-repo <PATH>` MAY appear on `plan`, `apply`, `explain`. Not on `init`, `agent`, `status`, or `skill`.
2. `--source-repo` requires `--host <HOST>` to be present in the same invocation. Validation: clap-level `requires` constraint.
3. `--source-repo` is mutually exclusive *within an invocation* with any future `--repo` / `--rev` resurrection (currently moot; no such flag exists).
4. The path must exist and be a directory. Implementation: `tokio::fs::metadata` (sync equivalent) before parser invocation; emit `miette` diagnostic with `.help("path must be an existing directory containing a spec/016 layout")` on failure.
5. The path is canonicalized (resolves symlinks, makes absolute) before being passed to the parser, ensuring `repository` provenance is reproducible.

**Rationale**: clap-derive supports `requires_ifs` / `conflicts_with` declaratively; minimum custom validation logic. Errors surface via the existing `miette` diagnostic chain.

---

## D8 — Test coverage strategy

**Decision**: Three layers of test coverage:

1. **Unit tests** (in `src/cli/args.rs`'s `#[cfg(test)]` module): clap argument parsing — `--source-repo` accepted on plan/apply/explain, rejected elsewhere; `--host` requirement; mutual-exclusion edge cases.
2. **Integration tests** (`tests/integration/test_stateless_{plan,apply,explain}.rs`): full command invocation via `assert_cmd` or `Command::cargo_bin`, exercising real source-repo loading from a `tempfile::TempDir`-built spec/016 layout. Cover (a) clean git checkout → SHA provenance, (b) dirty git working tree → `(stateless+dirty)` provenance, (c) non-git directory → `(stateless)` provenance, (d) missing `--host` error, (e) non-directory path error, (f) stateless apply on host with init'd state — assert `desired_state.repository`/`requested_ref` byte-identical pre/post (SC-009).
3. **Per-example integration tests** (`tests/integration/test_examples_<NN>_<slug>.rs`): for each of the five examples, parse via `Repository::load`, assert resolved service catalog contains expected service ids, assert example root carries `README.md`, run `core-ops plan --source-repo <example>` via `assert_cmd` and assert exit 0 plus non-empty stdout.

**CI cost**: 8 new tests × ~50ms median = ~400ms aggregate added to `cargo test`. Acceptable.

**No new VM-backed scenarios** required (exemption recorded; see plan.md Constitution Check).

---

## D9 — Release governance bump baseline

**Decision**: Spec language baseline updated from "2.1.0 → 2.2.0" to "current Cargo.toml → next minor". Master is currently at v2.1.1 (PR #31's auto-promote). The fragment declares `release_intent: minor`. The `core-ops-release validate --base-ref master` step is authoritative on whether the spec/016 example deletions force a `major` bump per the inferred-bump rules.

**Rationale**: Baseline arithmetic is a moving target during a feature branch's life; the spec/011 governance machinery handles it. The fragment's `release_intent` field is what matters; the `--base-ref` validator computes the required bump from the actual diff.

**Implementation note**: When updating `Cargo.toml`, set the version to `2.2.0` *or* run `cargo run --bin core-ops-release -- validate --base-ref master` first to see what the validator infers, then declare accordingly.

---

## D10 — Spec/016 examples and the migration script

**Decision**: `scripts/migrate-legacy-source-repo.sh` is independent of the spec/016 example fixtures and is unaffected by their removal. The script transforms a legacy-layout source repo into the formalized layout; it does not consume the example fixtures as inputs.

**Verification**: Read `scripts/migrate-legacy-source-repo.sh` during the implementation phase to confirm no example-path references; document in `tasks.md` if any are found and update accordingly.

---

## Open implementation questions deferred to /speckit.tasks

None. All technical decisions above resolve cleanly. The implementation tasks emitted by `/speckit.tasks` will follow the project structure in plan.md without requiring further clarification rounds.
