# Contract: `--source-repo` CLI flag

**Phase**: 1 | **Spec**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md)

The `--source-repo <PATH>` flag is the user-facing surface of stateless mode. This contract specifies its shape, validation, mutual exclusion, error semantics, and help-text expectations. Implementation lives in `src/cli/args.rs`, `src/cli/plan.rs`, `src/cli/apply.rs`, and `src/cli/explain.rs`.

---

## Acceptance: command coverage

The flag MUST be accepted by exactly these subcommands:

| Subcommand | Accepts `--source-repo`? | Mode semantics |
|------------|--------------------------|----------------|
| `core-ops plan` | YES | Read-only against persisted controller config; honors `--audit-dir` when explicitly set. |
| `core-ops apply` | YES | Mutates host state; writes audit + status snapshot with path-based provenance; init'd `desired_state.*` unchanged. |
| `core-ops explain` | YES | Read-only; no writes anywhere. |
| `core-ops init` | NO — init takes `<repository>` as a positional argument with separate semantics. |
| `core-ops agent` | NO — timer-driven; requires persisted tracking. |
| `core-ops status` | NO — reads only persisted state. |
| `core-ops skill` | NO — generates skill bundles; not source-repo-driven. |

Adding the flag to a non-listed subcommand is spec drift.

---

## Validation rules

1. **Argument shape**: `--source-repo <PATH>` where `<PATH>` is a filesystem path string. Implementation: clap `value_parser = clap::value_parser!(PathBuf)`.

2. **`--host` requirement**: When `--source-repo` is present, `--host <HOST>` MUST also be present. Stateless mode has no host fallback; rejection happens at clap-level via `requires("host")`.

3. **Path existence and shape**: `<PATH>` MUST be an existing directory. Validation runs *before* the parser:
   - Path does not exist → exit non-zero with `error: --source-repo path does not exist: <path>`.
   - Path exists but is not a directory → exit non-zero with `error: --source-repo path is not a directory: <path>`.
   - Path is a directory but the parser rejects its layout (legacy artifacts, missing services/, etc.) → exit non-zero with the existing parser diagnostic chain.

4. **No git-URL fallback**: A value like `https://github.com/foo/bar.git` is treated as a path string and fails at the existence check. There is no implicit URL parsing; that is `core-ops init`'s job.

5. **Canonicalization**: The provided path is canonicalized (symlinks resolved, made absolute) before being recorded as `desired_state.repository` provenance.

6. **Mutual exclusion within an invocation**: `--source-repo` does not conflict with any flag currently on `plan`/`apply`/`explain`. (No legacy `--repo` / `--rev` flag exists to conflict with — they were removed in spec/015.)

7. **Coexistence with init'd controller state**: The flag's presence in a single invocation is independent of whether `core-ops init` has been previously run on the host. Per the 2026-05-05 clarification (Q2), stateless invocations execute regardless of init'd state and never mutate `desired_state.repository` / `desired_state.requested_ref` of the persisted configuration (FR-013, SC-009).

---

## Provenance recording (stateless `apply` only)

When `core-ops apply --source-repo <PATH>` succeeds, the audit record and the status snapshot record provenance per the 2026-05-05 clarification (Q3):

| Source path state | `desired_state.repository` | `desired_state.requested_ref` |
|-------------------|----------------------------|-------------------------------|
| Non-git directory | `<canonicalized absolute path>` | `(stateless)` |
| Git working tree, dirty | `<canonicalized absolute path>` | `(stateless+dirty)` |
| Git working tree, clean at commit `abc1234…` | `<canonicalized absolute path>` | `<full 40-char SHA>` |

Stateless `plan` and `explain` write no persisted provenance (they bypass `/var/lib/core-ops/`). Stateless `plan` MAY write a plan-audit record to an operator-specified `--audit-dir`, with the same provenance shape as above (per 2026-05-05 clarification Q4).

---

## Help-text contract (FR-016)

The `--help` output for each accepting subcommand MUST include:

1. The flag name and value type: `--source-repo <PATH>`.
2. A one-line description: e.g., `Use a filesystem path as the source of desired state, bypassing the persisted init'd configuration.`
3. The `--host` co-requirement: `Requires --host. The init'd mode (no flag) sources from persisted state set by 'core-ops init'.`
4. A pointer to the canonical init'd-mode workflow: `For long-lived tracking, run 'core-ops init <repo> <ref>' once and omit --source-repo on subsequent invocations.`

The help text is part of the user contract; changing its shape between this spec and a future iteration requires a SemVer bump per Principle 9 (Conservative Public Evolution).

---

## Error semantics

| Condition | Exit code | Stderr |
|-----------|-----------|--------|
| `--source-repo` set, `--host` missing | clap default (2) | clap-generated `error: the following required arguments were not provided: --host <HOST>` |
| `<PATH>` does not exist | 64 (`EX_USAGE`) | `error: --source-repo path does not exist: <path>` (miette-rendered with help text) |
| `<PATH>` is not a directory | 64 | `error: --source-repo path is not a directory: <path>` |
| `<PATH>` is a directory but layout is invalid | 65 (`EX_DATAERR`) | existing parser diagnostic chain via `LayoutError` (`src/core/errors.rs`) |
| Git ref detection subprocess fails (`git -C <path> ...`) | continues with `(stateless)` fallback; logs miette warning to stderr | `warning: git ref detection failed for <path>; recording as non-git source` |
| `--source-repo` used on `init`/`agent`/`status`/`skill` | clap default (2) | clap-generated `error: unexpected argument '--source-repo' found` |

Exit codes follow the pattern already established in `scripts/migrate-legacy-source-repo.sh` (64 = usage, 65 = data, 66 = path) for cross-tool consistency.

---

## Test coverage (FR-006, plan.md D8)

Implementations of this contract MUST be covered by:

1. **clap unit tests** (`src/cli/args.rs::tests`): each acceptance case in the table above, plus rejection cases.
2. **Integration tests** (`tests/integration/test_stateless_{plan,apply,explain}.rs`): real `cargo run --bin core-ops` invocations against `tempfile::TempDir`-built source repos, asserting exit codes and stderr substrings.
3. **Per-example integration tests** (`tests/integration/test_examples_<NN>_<slug>.rs`): each of the five examples invoked via `core-ops plan --source-repo examples/<NN-slug> --host <host>` and asserted exit 0.

---

## Future evolution

- A `--repo` short alias is intentionally NOT added in this slice — `--repo` was removed by spec/015 and reintroducing the spelling would cause user confusion. If a shorter form is wanted later, it should be a new clap alias added explicitly with documentation.
- A reverse alias (e.g., environment variable `CORE_OPS_SOURCE_REPO`) is out of scope for this slice; spec/004 / spec/006 followed an environment-variable-free model.
- Stateless mode for `agent` is explicitly out of scope (architectural mismatch). The follow-ups document records no plan to add it.
