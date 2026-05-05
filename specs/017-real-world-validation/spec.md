# Feature Specification: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Feature Branch**: `017-real-world-validation`
**Created**: 2026-05-05
**Status**: Draft
**Input**: User description: "Validate the spec/016 source-repository layout against five real-world homelab setups, produce documented examples under top-level `examples/`, and classify any friction encountered as amendment-now / workaround-with-doc / defer-to-spec-018. Closes the 'rich, documented real-life examples' and 'QnA for known limitations' bullets in docs/follow-ups.md lines 87–99."

> **Scope expansion (locked 2026-05-05)**: the validation iteration's first finding is that `core-ops plan` and `core-ops apply` cannot be invoked against a source-repository directory without first running `core-ops init` (which writes persistent state to `/var/lib/core-ops/`). This blocks all five planned examples plus the four spec/016 example "Try it" snippets that ship referencing a non-existent `--source-repo` flag in v2.0.0. The slice is therefore expanded to:
>
> 1. Add a stateless `--source-repo <PATH>` flag to `plan` and `apply` for one-off invocations against a filesystem path, bypassing `init` and the persisted controller configuration.
> 2. Delete the four spec/016 in-tree example fixtures — superseded by this slice's real-world examples under top-level `examples/`.
> 3. Clean up stale CLI documentation that references the long-removed `--repo` / `--rev` flags.
>
> Items (1) and (2) cross the original "validation, not feature" framing; the trade is intentional and locked.

## Clarifications

### Session 2026-05-05

- Q: Should stateless apply require an explicit safety flag (e.g., `--confirm-stateless`) beyond `--source-repo`? → A: No. Standard apply semantics; the explicit `--source-repo` flag plus path-based provenance in audit + status snapshots is sufficient explicit intent. No additional confirmation ceremony.
- Q: On a host where `core-ops init` has already been run, can stateless `--source-repo` invocations execute, or do they require teardown of the init'd state first? → A: Coexistence. Stateless invocations always execute regardless of init'd state. Init'd `desired_state.repository` / `desired_state.requested_ref` are never mutated by stateless; the two modes coexist on the same host, distinguished only in audit/provenance.
- Q: When `--source-repo` points at a git working tree, what should `desired_state.requested_ref` record in audit + status snapshots? → A: Detected git commit when the path is a clean git checkout at a known commit; `(stateless+dirty)` sentinel when the working tree has uncommitted changes; `(stateless)` sentinel when the path is not a git repository at all. Honors Principle 12 (behavior is traceable to the desired-state revision actually applied) without misrepresenting state.
- Q: Should stateless `plan` honor an explicit `--audit-dir` flag (write the plan audit record to the operator-specified destination) or ignore it under the "pure read-and-render" framing of FR-012? → A: Honor `--audit-dir` when explicitly provided. The operator passing the flag is explicit consent per Principle 4. The "no `/var/lib/` writes" guarantee is preserved separately. Init'd plan and stateless plan behave identically with respect to `--audit-dir`.
- Q: Should `core-ops explain` also accept `--source-repo`, or stay init-only alongside `agent` and `status`? → A: Add `--source-repo` to `explain` in this slice. Symmetric with stateless `plan`/`apply` for the read-only inspection use case (debugging a working tree before commit, evaluating an arbitrary path's expansion). Pure-read so persistence semantics are simpler than apply. `agent` and `status` remain init-only by architectural fit (timer-driven; reads persisted state).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - First-time operator runs a real example with one command (Priority: P1)

A newcomer who has just installed CoreOps visits the project README, finds the "Real-World Examples" section, picks a setup that resembles the workload they want to run, clones the repo (or downloads the example), and runs **a single command** that produces a populated `core-ops plan` against that example — without `init`, without writing persisted state to `/var/lib/`, without committing to anything.

**Why this priority**: This is the entry path for adoption. The cost of evaluating an example must be ~10 seconds (one command), not 30 minutes (clone, init, force, switch examples, force again). Every minute of friction at first impression bleeds adoption.

**Independent Test**: A reviewer who has never seen CoreOps before clones the repo, reads `examples/<setup>/README.md`, runs the suggested `core-ops plan --source-repo examples/<setup> --host <example-host>` invocation, and gets a non-empty plan with no parser errors. No `init`, no state files written, no `--force` needed to pivot to a different example.

**Acceptance Scenarios**:

1. **Given** a fresh CoreOps install (no prior `init`) and the published `examples/01-caddy-whoami/`, **When** the operator runs `core-ops plan --source-repo examples/01-caddy-whoami --host example`, **Then** the command exits 0, emits a plan listing the Caddy container unit and any associated network/volume, and writes nothing to `/var/lib/core-ops/`.
2. **Given** the operator has just evaluated `examples/01-caddy-whoami` via stateless plan, **When** they pivot to `examples/02-nextcloud` and run the same flag against it, **Then** the command succeeds without `--force`, without prior teardown, and without surfacing any "controller configuration already exists" error.
3. **Given** any of the five published examples, **When** the operator reads the example's README, **Then** they see (a) a one-line setup description, (b) at least one public upstream source cited for the design, (c) a service-by-service intent table, (d) any known limitations encountered during translation, (e) a "Try it" code block using the stateless `--source-repo` invocation as a single-command demonstration.

---

### User Story 2 - Operator authors and iterates on their own setup without committing first (Priority: P2)

An operator with an existing homelab wants to migrate to CoreOps. They scan the `examples/` roster for an analog, copy the closest example to a working directory, and iterate locally — running `core-ops plan --source-repo ./my-source-repo --host myhost` repeatedly as they edit files, **before** they have a git repo, **before** they've committed anything, **before** they've decided whether to adopt CoreOps. When ready, they `git init` and `core-ops init` to switch into long-lived tracking mode.

**Why this priority**: Authoring requires fast iteration. Forcing `git commit` + `core-ops init --force` between every experiment kills the inner loop. The stateless mode is the authoring substrate; the init-tracked mode is the production substrate. Both must coexist.

**Independent Test**: Pick any of the five examples, copy its directory tree to a non-git scratch location, change the host name and one service config, and verify `core-ops plan --source-repo <scratch> --host <new-host>` succeeds without git presence and without writing persisted state.

**Acceptance Scenarios**:

1. **Given** `examples/02-nextcloud/`, **When** the operator copies it to `~/my-source-repo/` (no `git init`), renames the host directory from `hosts/example/` to `hosts/myhost/`, and updates `host.yaml` accordingly, **Then** `core-ops plan --source-repo ~/my-source-repo --host myhost` succeeds.
2. **Given** an operator who encounters a friction documented as "workaround-with-doc" in this spec's synthesis table, **When** they apply the documented workaround in their stateless source repo, **Then** the workaround unblocks parsing without further escape hatches.
3. **Given** an operator who has finished iterating with stateless mode and now wants to commit, **When** they `git init && git commit && core-ops init <path> <ref>`, **Then** subsequent `core-ops plan` (without `--source-repo`) sources from the persisted state and produces an equivalent plan to the last stateless run against the same tree.

---

### User Story 3 - Stateless apply for one-off convergence and recovery (Priority: P2)

An operator needs to converge a host to a desired state expressed in a local directory without committing to long-lived `init`-tracked operation. Examples: a recovery scenario where the persisted `init` state is corrupt and the operator has a known-good source-repo checkout; a one-off bootstrap where the host should converge once from a directory and then be re-init'd with a different repo; a CI/test scenario that must apply a synthetic source repo to a VM without persistent state.

**Why this priority**: Stateless plan without stateless apply is half-finished — the operator can preview but cannot execute. This is P2 (not P1) because plan is the more frequent first-impression surface, but apply must follow in the same slice or the feature is not coherent.

**Independent Test**: On a host with no prior `core-ops init`, run `core-ops apply --source-repo <PATH> --host <HOST>`. Verify (a) the apply succeeds and produces audit records, (b) `/var/lib/core-ops/` controller-configuration state is not written by the stateless invocation (or is written with a sentinel marking the stateless source), (c) a subsequent `core-ops status` reports the applied state with provenance pointing to the path-based source rather than a git URL.

**Acceptance Scenarios**:

1. **Given** a fresh host with no prior `init`, **When** the operator runs `core-ops apply --source-repo /path/to/repo --host edge-01`, **Then** apply succeeds, audit records are produced, and host state is converged.
2. **Given** apply has run statelessly, **When** the operator runs `core-ops status`, **Then** status reports the applied state with provenance referencing the path-based source clearly distinguished from a git-URL-based source.
3. **Given** the operator subsequently runs `core-ops init <git-url> <ref> --force` and then `core-ops plan`, **Then** plan produces a normal init'd-mode plan without confusion from the prior stateless apply.

---

### User Story 4 - Future spec author grounds amendments in validation evidence (Priority: P3)

A future spec author considering an amendment to spec/016 (e.g., adding a templating layer, relaxing the payload-kind whitelist, introducing a new reserved-prefix convention) consults the synthesis table. They identify which gaps are blocking ≥2 examples (escalation candidates), which are workaround-with-doc, and which are deferred. The decision to amend is grounded in evidence, not speculation.

**Why this priority**: Spec/016 just shipped at v2.0.0. Future amendments must clear a higher bar than informal anecdote. P3 because it serves a future workflow.

**Independent Test**: Open this spec's synthesis table, count A/B/C classifications, verify each cited friction names the affected examples plus a rationale, and confirm any A-classified friction is escalated to a follow-up spec.

**Acceptance Scenarios**:

1. **Given** the published synthesis table, **When** a future spec author asks "is friction X worth amending?", **Then** the table answers with affected-example count, classification, and rationale, sufficient to decide without re-running the validation iteration.

---

### Edge Cases

- **Stateless invocation on a host that already has init'd controller configuration**: The stateless `--source-repo` invocation MUST NOT mutate the persisted `init`-tracked configuration. It MAY produce its own provenance record (audit, status snapshot) but the `desired_state.repository` / `desired_state.requested_ref` of the init'd configuration MUST remain unchanged. After the stateless run, a subsequent `core-ops plan` (without flag) MUST behave as before the stateless run — no detached state, no rollback ambiguity.
- **Stateless apply with no `--host` flag**: There is no persisted host identity for stateless mode to fall back on; the command MUST fail with a clear message naming `--host` as required.
- **Path that is not a directory**: Stateless mode MUST fail with a clear message; no fallback to interpreting the path as a git URL.
- **Path that is a directory but not spec/016-conformant**: Parser errors surface as today; the stateless flag does not change parser strictness.
- **Real-world setup cannot be expressed without a non-payload file inside `services/<svc>/`** (parser rejects per `src/io/repo.rs`). → Reserved-prefix subdir (`_*`) is the documented escape; if even that doesn't suffice, classify as A (amend-now) and escalate to a follow-up spec.
- **Upstream's compose.yml or example config is licensed incompatibly with this repository's AGPLv3+**. → Derive own Quadlet equivalents inspired by the upstream design; cite upstream as the reference; do not copy YAML verbatim.
- **A setup's secrets pattern requires external host-managed state** (e.g., a sops/age credstore, a `LoadCredentialEncrypted` source). → Document the host-side prerequisite in the example README; commit only stub references with placeholder names; never commit fake or real secrets.
- **A real-world setup uses container images hosted on a registry with strict pull rate limits**. → Integration tests parse only; the parser does not pull images.
- **An asciinema-style demo is requested mid-iteration**. → Out of scope; the asciinema follow-up at `docs/follow-ups.md` line 109 remains open.

## Requirements *(mandatory)*

### Functional Requirements

#### Real-world examples

- **FR-001**: The repository MUST publish exactly five examples under `examples/<NN-slug>/` with slugs `01-caddy-whoami`, `02-nextcloud`, `03-immich`, `04-traefik-authelia`, `05-observability`. Each slug pairs a sequence number with a kebab-case identifier of the headlining service or theme.
- **FR-002**: Each example MUST conform to the spec/016 source-repository layout: a root `README.md`; a `services/` directory containing one or more service subdirectories; a `hosts/` directory containing at least one example host. Per-service directories MUST contain only `service.yaml` (optional) and the payload-kind subdirectories `quadlet/`, `systemd/`, `config/` accepted by the parser.
- **FR-003**: Each example MUST parse cleanly via `core-ops plan --source-repo examples/<setup> --host <example-host>` — exit 0, non-empty plan output — without manual edits and without prior `core-ops init`.
- **FR-004**: Each example's `README.md` MUST contain (a) a one-line setup description, (b) at least one public upstream source cited as the design reference, (c) a service-by-service intent table, (d) any known limitations encountered during translation, (e) an explicit declaration that any CLI-output snippets shown are illustrative and not snapshot-tested, (f) a "Try it" code block using the stateless `--source-repo` invocation as a single-command demonstration.
- **FR-005**: This spec MUST publish a synthesis table listing every friction encountered during translation. Each entry MUST carry: friction description, affected example list, classification A/B/C, rationale, and recommended action. Classification semantics:
  - **A — amend-now**: ≥2 examples blocked by the same gap; the iteration escalates the gap to a follow-up spec rather than landing parser changes inline.
  - **B — workaround-with-doc**: friction is real but addressable via reserved-prefix subdirs, host-side preparation, drop-in tricks, or other documented patterns. The workaround is documented in the affected example's README.
  - **C — defer-to-spec-018**: friction is acknowledged but neither blocking nor addressable in this slice; named for a future iteration.
- **FR-006**: The repository MUST contain per-example integration tests at `tests/integration/test_examples_<NN>_<slug>.rs` that load each example through the parser and assert (a) parse succeeds, (b) the resolved service catalog contains the expected service identifiers, (c) the example root carries a `README.md`, (d) `core-ops plan --source-repo <example> --host <example-host>` exits 0 (gated on the `--source-repo` flag landing in this slice). New tests MUST be registered in `tests/integration/mod.rs`.
- **FR-007**: The root `README.md` MUST add a `## Real-World Examples` section between `## First Interaction` and `## Installation (Current Phase)`, linking each of the five examples with a one-line purpose statement.
- **FR-008**: All hostnames published in `examples/` MUST use RFC 2606 reserved domains (`*.example.com`, `*.example.org`, `*.test`, `*.invalid`, `*.localhost`). All IP literals MUST use RFC 5737 documentation ranges (`192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`).
- **FR-009**: No file under `examples/` MAY contain operator-private values: the operator's real domain (`*.not.one`), real IP `192.168.1.2`, real GCloud DNS challenge credentials, or any other operational data sourced from the operator's private homelab repository (`~/code/ulthar/repo/`). Ulthar is consulted as a research data point only.

#### Stateless `--source-repo` CLI mode

- **FR-010**: `core-ops plan` MUST accept a new `--source-repo <PATH>` flag that selects a filesystem-directory source of desired state, bypassing the persisted controller configuration written by `core-ops init`. The flag requires `--host <HOST>` to be present (no host fallback exists in stateless mode). The flag MUST execute regardless of whether the controller is init'd; init'd `desired_state.*` is never read or mutated during a stateless invocation. Init'd-mode and stateless-mode coexist on the same host, mutually exclusive only within a single command invocation (per the 2026-05-05 clarification).
- **FR-011**: `core-ops apply` MUST accept the same `--source-repo <PATH>` flag with equivalent semantics: bypass the persisted controller configuration, accept the directory as the desired-state source, require `--host`. No additional safety/confirmation flag is required beyond `--source-repo` and `--host`; the audit chain plus path-based provenance provides the traceability trail (per the 2026-05-05 clarification).
- **FR-011a**: `core-ops explain` MUST accept the same `--source-repo <PATH>` flag with read-only semantics: bypass the persisted controller configuration, source the desired state from the directory, require `--host`, write nothing to `/var/lib/core-ops/` (per the 2026-05-05 clarification, symmetric with stateless `plan`). `core-ops agent` and `core-ops status` remain init-only — `agent` is timer-driven and requires persisted tracking; `status` reads only persisted state.
- **FR-012**: Stateless `plan` MUST NOT write to `/var/lib/core-ops/` or any other persisted controller-state location. It is a pure read-and-render operation with respect to controller state. Stateless `plan` MUST honor an explicitly-provided `--audit-dir <DIR>` flag exactly as init'd `plan` does — writing the plan audit record to the operator-specified destination. The operator passing `--audit-dir` is explicit consent (per the 2026-05-05 clarification) and does not violate the no-`/var/lib/` guarantee.
- **FR-013**: Stateless `apply` MUST converge host state and write audit records as today. The persisted `desired_state.repository` and `desired_state.requested_ref` of any existing init'd configuration MUST remain unchanged by a stateless apply. Provenance written by stateless apply (status snapshot, audit) MUST clearly distinguish the path-based source. `desired_state.repository` records the absolute path. `desired_state.requested_ref` records (per the 2026-05-05 clarification):
  - the detected git commit SHA when the path is a clean git checkout at a known commit (full traceability);
  - the sentinel `(stateless+dirty)` when the path is a git working tree with uncommitted changes (signals the applied state was not a clean revision);
  - the sentinel `(stateless)` when the path is not a git repository at all.
- **FR-014**: Stateless mode MUST fail with an actionable error if `<PATH>` is not a directory or is not a spec/016-conformant source repository. No fallback interpretation as a git URL.
- **FR-015**: Stateless mode MUST be expressible without git: a non-git directory containing a valid spec/016 layout MUST work end-to-end through `plan` and `apply`.
- **FR-016**: The CLI help text for `plan`, `apply`, and `explain` MUST document the new `--source-repo` flag, its bypass relationship with the init'd-mode default, and the `--host` requirement. The help text MUST also link to the canonical `init`-then-`plan` workflow for the long-lived case so users can choose the right mode.

#### Spec/016 example removal & supersession

- **FR-017**: This change MUST remove the four in-tree example fixtures at `specs/016-source-repository-layout/examples/{01-minimal-single-service,02-variant-config-root,03-multi-unit-with-dropins,04-host-overlay}/`. They are superseded by `examples/<NN-slug>/` published under FR-001.
- **FR-018**: This change MUST update spec/016's `spec.md` (FR-023 and any user-story references) and `tasks.md` (T101–T104 carry historical "[X] [SUPERSEDED by spec/017]" annotations) to record the supersession without rewriting historical task records. The decision file `decision_examples-are-layout-shape-fixtures.md` (memory) is updated by separate operator action; this spec does not write memory files.
- **FR-019**: Any integration test that loaded the four spec/016 example fixtures MUST be removed or repointed at the spec/017 examples. Test inventory MUST be net-positive (≥5 new tests, equal or fewer total deletions of the spec/016-example tests).

#### Stale-doc cleanup

- **FR-020**: This change MUST remove or update stale CLI documentation that references the long-removed `--repo` and `--rev` arguments:
  - `docs/follow-ups.md`: the "Init Command" section's "currently expects" / "shall be introduced" / "remove the `repo` and `rev` arguments" prescriptions are now historical (the change shipped in spec/015). The paragraphs are removed or rewritten as a brief historical note. Other items in the same section that remain valid follow-ups (e.g., `rollback-plan-only` re-homing) are preserved.
  - `docs/development.md` line 228: the literal `core-ops plan --repo <repo> --rev <rev>` example is updated to use `--source-repo` (stateless mode) or `core-ops init` then `core-ops plan` (init'd mode), whichever is contextually appropriate.
  - `infra/repo/README.md` lines 32, 35, 38: the `core-ops plan --repo file:///... --rev demo-uat-vN` examples are updated to the current CLI surface.
  - Historical specs (`specs/001-*`, `specs/007-*` quickstart files) are NOT modified — they are time-capsule artifacts of the spec at the time it was authored.

#### Release governance

- **FR-021**: This change MUST update `Cargo.toml` to a new `minor` version (`2.1.0` → `2.2.0`), add `changes/017-real-world-validation.md` declaring `release_intent: minor`, and re-render `CHANGELOG.md` via `cargo run --bin core-ops-release -- changelog --write`. If the inferred-bump validator detects deletions in `specs/016-source-repository-layout/examples/` as `major` per spec/011, the declared intent is bumped to `major` and the version becomes `3.0.0` — the validator's verdict is authoritative.

### Key Entities

- **Example setup**: A directory under `examples/<NN-slug>/` carrying a self-contained source-repository expression of one real-world deployment topology, plus README documentation explaining intent, sources, pressure axis, and known limitations.
- **Friction record**: An entry in the synthesis table identifying a specific gap encountered during translation, naming the affected example(s), classification (A/B/C), rationale, and recommended action.
- **Pressure axis**: A design property each example is chosen to exercise (single-Container baseline; multi-Container with intra-service network and persistent storage; GPU device passthrough plus multi-network membership; cross-service ForwardAuth composition; host-scope sidecars with `/proc` and `/sys` bind mounts and scrape-config templating).
- **Stateless invocation**: A `core-ops plan` or `core-ops apply` invocation using `--source-repo <PATH>` as the desired-state source, bypassing the persisted controller configuration written by `core-ops init`. Distinguished in provenance by the path-based `desired_state.repository` and `(stateless)` sentinel `requested_ref`.

## Constitution Alignment *(mandatory)*

- **Functional core vs. side effects**: Stateless mode adds a new entry-point boundary (path → in-memory `EvaluationInput`) but reuses the existing parser, planner, and applier. No new core or side-effect surfaces beyond the existing apply mutation paths.
- **Declarative state model**: Examples are pure declarative artifacts. The stateless flag changes the source-of-truth resolution but not the in-memory representation; `EvaluationInput` is identical between init'd and stateless modes.
- **Idempotence & convergence**: Stateless apply has the same convergence guarantees as init'd apply against the same tree. Re-running stateless apply against the same path is idempotent.
- **Explicit effects/failures**: Stateless mode's persistence semantics are documented explicitly (FR-013): apply writes audit and status snapshots; plan writes nothing. The path-based provenance sentinel makes the source mode visible in `core-ops status`.
- **Observability**: Synthesis table is the validation observability surface. Stateless invocations produce normal audit records distinguished by their provenance source field.
- **Provenance & traceability**: Stateless invocations record path-based provenance distinct from git-URL provenance, preserving Principle 12's traceability invariant: runtime behavior is traceable to both reconciler revision and desired-state revision (where revision is the path's commit if git-managed, else `(stateless)` sentinel).
- **Safe defaults**: The default mode remains init'd; stateless requires the explicit `--source-repo` flag. Safe defaults preserved.
- **Compatibility**: Init'd-mode behavior is unchanged. Existing `core-ops plan` / `apply` invocations without `--source-repo` continue to source from persisted state. The flag is additive.
- **Release version policy**: Adds CLI surface (`--source-repo`) — `minor` per spec/011 inferred-bump rules. Removes spec/016 example directories — may trigger `major` per the inferred-bump rules' "deleted source" classification (spec/016 examples are not under `src/` but the validator's exact rules govern). Declared intent is `minor`; the `core-ops-release validate` step is authoritative on the final bump.
- **Release intent artifact**: `changes/017-real-world-validation.md` declares `release_intent: minor` (or `major` if the validator demands). The fragment lists CLI additions, example additions, spec/016 example removal, and stale-doc cleanup.
- **Changelog discipline**: An `[Unreleased]` entry is rendered via `core-ops-release changelog --write`; the post-merge promote step (shipped in v2.1.0) moves it to `[2.2.0]` (or `[3.0.0]`) automatically.
- **Test contract**: Per-example integration tests assert parser success and structural shape via the new `--source-repo` flag. New unit tests for stateless argument parsing, mutual-exclusion validation, path resolution. New integration test for stateless apply against a synthetic source repo (no real podman pull). `cargo test` and `cargo clippy --all-targets -- -D warnings` MUST pass before merge. **VM-backed scenario assessment**: stateless mode introduces a CLI entry-point variation but no new mutation classes — the actual host-state changes performed by stateless apply are identical to those performed by init'd apply against the same tree. Existing apply VM scenarios (`tests/fixtures/verification/scenarios/`) remain valid; stateless mode is exercised at the unit + integration test layer. Per Principle 10's exemption clause, no new VM-backed scenario is required for this feature; this exemption is recorded explicitly here as the documented justification.
- **Regenerability**: Examples are derivative of public upstream sources cited per-example. The synthesis table can be re-derived from translation artifacts. Stateless mode's semantics are spec'd here and tested by integration cases.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new operator can clone the repository and run `core-ops plan --source-repo examples/01-caddy-whoami --host example` successfully — exit code 0, non-empty plan, no `init` required, no writes to `/var/lib/core-ops/` — without modifying any file.
- **SC-002**: 100% of frictions surfaced in any example's README appear as entries in the synthesis table with an A/B/C classification, an affected-example list, and a rationale.
- **SC-003**: All five examples parse cleanly via the parser through the new stateless flag, asserted by per-example integration tests; `cargo test` adds at least five new test cases that pass and the aggregate test suite remains green.
- **SC-004**: The two follow-up bullets in `docs/follow-ups.md` lines 87–99 (Source Repository UX → "Rich, documented real-life examples" and "QnA for known limitations") are removable from `docs/follow-ups.md` upon merge. The "Init Command" section paragraphs that prescribe the (now-shipped) `--repo`/`--rev` removal are also removable.
- **SC-005**: A grep across `examples/` for the strings `not.one`, `ulthar`, `192.168.1.2`, and any GCloud DNS credential marker returns zero matches.
- **SC-006**: Each example's `README.md` cites at least one public upstream source URL.
- **SC-007**: The root `README.md` carries a `## Real-World Examples` section linking each of the five examples with a one-line purpose; rendered Github README shows the section between `## First Interaction` and `## Installation (Current Phase)`.
- **SC-008**: A `core-ops plan --source-repo <PATH>` invocation against a fresh host (no `core-ops init` ever run) succeeds without surfacing any "controller configuration not initialized" error.
- **SC-009**: A stateless apply against a host with prior init'd configuration MUST NOT mutate the init'd `desired_state.repository` or `desired_state.requested_ref` — verified by an integration test that init's, statelessly applies, then asserts the persisted desired-state fields are byte-identical to pre-stateless.
- **SC-010**: After this change merges, the four `specs/016-source-repository-layout/examples/` subdirectories are absent; `cargo test` passes with the spec/017 example tests as the sole example-fixture coverage.
- **SC-011**: `core-ops explain --source-repo <PATH> --host <HOST> <object-id>` succeeds against any of the five published examples without prior `core-ops init` and writes nothing to `/var/lib/core-ops/`.

## Assumptions

- The five-setup roster is operator-confirmed and frozen for this slice: Caddy + whoami; Nextcloud (community multi-container) + Postgres + Redis + Traefik; Immich server + db + redis + ML + Traefik; Traefik + Authelia + protected backend; Prometheus + Grafana + node-exporter + cadvisor.
- Asciinema recording is deferred. The follow-up at `docs/follow-ups.md` line 109 remains open.
- Ulthar (`~/code/ulthar/repo/hosts/ulthar/`) is consulted as a research data point only.
- Examples are parse-only deliverables. CI does not pull images.
- Friction encountered does not block this slice's merge unless ≥2 of 5 examples are blocked by the same gap; in that case the validation iteration escalates the gap to a follow-up amendment spec rather than landing parser changes inline.
- The post-merge `core-ops-release promote` step (shipped in v2.1.0) handles the `[Unreleased]` → `[<version>]` transition and fragment cleanup automatically.
- Stateless mode's exact provenance representation (`(stateless)` sentinel value, path-vs-URL distinguisher) is locked during `/speckit.plan` or `/speckit.clarify` based on a review of `src/core/types.rs` `DesiredStateProvenance` shape.
- Stateless apply does not introduce new behavioral mutation classes; existing apply VM-backed scenarios remain authoritative for the mutation semantics.
- Removal of `specs/016-source-repository-layout/examples/` does not break any release-governance validation rule beyond the standard `major`-on-deletion check; the release fragment may need to declare `major` if the validator considers spec example directories as governed source. The validator's verdict is authoritative.
