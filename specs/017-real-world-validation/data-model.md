# Data Model: Real-World Validation, Examples, and Stateless Source-Repo Mode

**Phase**: 1 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

This feature does not introduce new core types. It introduces (a) **value-level conventions** on the existing `DesiredStateProvenance.requested_ref` field for stateless invocations, and (b) a **markdown-table schema** for the friction-classification synthesis surface in spec.md.

---

## E1 — `DesiredStateProvenance.requested_ref` value conventions (extension)

**Existing struct** (unchanged): `src/core/types.rs:553-560`:

```rust
pub struct DesiredStateProvenance {
    pub repository: String,           // git URL or absolute filesystem path
    pub requested_ref: String,        // git ref name, SHA, or sentinel
    pub layout_version: Option<String>,
    // ... other fields unchanged
}
```

**New value conventions for `requested_ref`** (introduced by spec/017):

| Value shape | Meaning | When recorded |
|-------------|---------|---------------|
| `<40-char hex SHA>` | Specific git commit | (a) Init'd mode against a pinned ref. (b) Stateless mode when `--source-repo` is a clean git checkout at a known commit (per FR-013 + 2026-05-05 clarification Q3). |
| `<branch-name>` / `<tag-name>` | Symbolic git ref | Init'd mode when tracking a branch or tag; resolution-time SHA recorded separately in `repository_ref`. |
| `(stateless)` | Sentinel: non-git source | Stateless mode when `--source-repo` is a directory that is not a git working tree. |
| `(stateless+dirty)` | Sentinel: dirty git working tree | Stateless mode when `--source-repo` is a git working tree with uncommitted modifications, additions, deletions, or untracked files inside the source-repo path. |

**Disambiguation guarantee**: sentinel values begin with the `(` character. Per `git check-ref-format`, parentheses are not valid in a git ref name. Therefore sentinels cannot be confused with real refs in any consumer code that pattern-matches against `requested_ref`.

**`repository` value conventions for stateless invocations**: the canonical, symlink-resolved absolute path of the source-repo directory (from `std::fs::canonicalize`). This distinguishes path-based provenance from URL-based provenance unambiguously — git URLs always contain `:` (in `https://` or `user@host:`), while canonical filesystem paths begin with `/`.

**Compatibility**: existing fixtures and tests that hardcode `requested_ref` values use either branch names (e.g., `master`, `main`) or SHA-shaped strings. None of them use `(`-prefixed values. Adding the sentinels is a non-breaking value-level extension.

---

## E2 — Synthesis-table schema (markdown-rendered in `spec.md`)

The friction-classification synthesis table is the validation iteration's evidence base. It lives directly in `spec.md` as a markdown table, populated during the Translation phase (Phase 2 of `/speckit.tasks`) and reviewed during the Synthesis phase.

**Columns** (in order):

| Column | Type | Purpose |
|--------|------|---------|
| `Friction` | short prose | One-line description of the gap encountered. |
| `Affected examples` | comma-separated example slugs (e.g., `01-caddy-whoami, 03-immich`) | Which examples surfaced this friction. |
| `Classification` | enum: `A` / `B` / `C` | A = amend-now (≥2 examples blocked, escalate to follow-up spec); B = workaround-with-doc (default); C = defer-to-spec-018 (acknowledged, not addressed). |
| `Rationale` | short prose | Why this classification. For A: name the structural impossibility. For B: name the workaround. For C: name why deferral is acceptable. |
| `Action` | one of: `Escalate to spec/<NNN>` / `Documented in <example>/README.md` / `Tracked in docs/follow-ups.md` | Next step. |

**Validation rules**:
- Classification is exactly one letter from `A`, `B`, `C`. Values outside this set are spec drift.
- `A` rows MUST have `Action = "Escalate to spec/<NNN>"` referencing a real (or to-be-created) follow-up spec number. The escalation triggers a separate spec/<NNN> branch and PR.
- `B` rows MUST have at least one example slug listed under `Affected examples`, and that example's `README.md` MUST document the workaround under a "Known limitations" heading.
- `C` rows MUST have `Action = "Tracked in docs/follow-ups.md"` and the corresponding bullet MUST exist in the follow-ups document by the time the slice merges.

**Empty-table semantics**: an empty synthesis table (no rows) means no friction was encountered during translation, which is itself a finding — the spec/016 layout was sufficient as-shipped. SC-002 is trivially satisfied.

**Schema is enforceable but not tested**: this is a markdown table in a spec, not a structured data file. The validation rules above are lint-style invariants; deviation is caught by review, not by code. If future iterations want machine-checkability, the table can be promoted to a YAML or TOML data file with a separate validator binary; out of scope here.

---

## E3 — Per-example directory shape (already constrained by spec/016 parser)

This is restated from `spec.md` FR-002 for completeness, not as a new model. Each `examples/<NN-slug>/` directory MUST have:

```text
examples/<NN-slug>/
├── README.md                           # parser-tolerated at example root only
├── services/
│   └── <svc>/
│       ├── service.yaml                # OPTIONAL; required only if config-root != svc id
│       ├── quadlet/                    # Container, Volume, Network, Pod
│       ├── systemd/                    # Socket, Mount, Automount, Timer, Target, Path
│       └── config/                     # mapped to /etc/<config-root>/
└── hosts/
    └── <example-host>/
        ├── host.yaml
        └── <svc>/
            ├── quadlet/                # drop-ins only (no new base units)
            ├── systemd/                # drop-ins only
            └── config/                 # whole-file replacements
```

`<NN-slug>` ∈ {`01-caddy-whoami`, `02-nextcloud`, `03-immich`, `04-traefik-authelia`, `05-observability`}. `<example-host>` is per-example operator's choice (typically the headlining service name, e.g., `homelab.example.com` or simply `example`).

The parser at `src/io/repo.rs` rejects any other directory or non-payload file inside `services/<svc>/`; reserved-prefix subdirs (`_*`) are tolerated for documentation that needs to live inside a service directory. README at example root is fine; README inside `services/<svc>/` is rejected.
