# Data Model: Source Repository Layout Formalization

**Feature**: 016-source-repository-layout
**Date**: 2026-05-01 (revised after implementation scope review)
**Scope**: YAML schemas for the new manifests, and the **minimum** Rust type surface touched by this feature. Internal parser types stay inside `src/io/repo.rs`; the planner's input contract (`EvaluationInput`, `ServiceDefinition`, `HostOverlaySet`, etc.) is **preserved unchanged** to avoid cascading rewrites of the planner, applier, and tests.

> **Note on revision**: an earlier draft of this document proposed introducing `ServiceDefinition`, `HostOverlay`, `PayloadTree`, etc. to `src/core/types.rs` as replacements for the existing planner-input types. Implementation scoping showed that the existing types already absorb the new layout (the `QuadletType` enum covers both Quadlet and native systemd extensions; `ConfigFileSource.target_path` already encodes the resolved `/etc/<config-root>/...` path). Replacing those types would ripple through `EvaluationInput`, `EvaluatedArtifact`, the planner, and most integration tests for no semantic gain. This revision narrows the type surface accordingly.

---

## YAML schemas

### `services/<svc-id>/service.yaml` (optional)

```yaml
config-root: <string>   # required if file present; identifies the /etc/<root>/ directory
```

- File absent → `config-root` defaults to `<svc-id>`.
- Any key other than `config-root` → load fails (`UnknownServiceManifestKey`).
- Empty file or `{}` → load fails (the file is optional, but if present must declare `config-root`).

Rust deserializer:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ServiceManifest {
    config_root: String,
}
```

### `hosts/<host-id>/host.yaml`

```yaml
host: <string>             # MUST equal <host-id> directory name
services:                  # ordered list of service ids
  - <svc-id-1>
  - <svc-id-2>
```

Rust deserializer:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct HostManifest {
    host: String,
    services: Vec<String>,
}
```

`HostManifest` then validates that `host` equals the parent directory name (`hosts/<id>/host.yaml` → `manifest.host == "<id>"`). Mismatch fails with a `LayoutError::HostManifestParse` diagnostic.

---

## Rust type surface

### Existing types — **unchanged**

The following live in `src/core/types.rs` today and remain the planner's input contract. This feature does **not** modify them:

- `DesiredState` — top-level reconciler input/output.
- `ServiceCatalog`, `ServiceDefinition` — the parser's per-service output. `ServiceDefinition` continues to carry `name`, `artifacts: Vec<ArtifactSource>`, `base_dropins: Vec<DropInSource>`, `config_files: Vec<ConfigFileSource>`. These shapes already accept the new layout's data — the new parser populates them from a different on-disk layout, but the in-memory shape is identical.
- `HostDeclaration { host, services }` — direct deserialization of `host.yaml`.
- `HostOverlaySet { host, overrides, config_overrides }` — host-side drop-ins and config overlays.
- `EvaluationInput { host, catalog, overlays }` — what the parser returns to the planner.
- `ArtifactSource`, `DropInSource`, `ConfigFileSource`, `QuadletType`, etc. — supporting shapes.

`QuadletType` already includes `Container`, `Socket`, `SocketDropIn`, `ConfigFile`, `Mount`, `Automount`, `Pod`, `Volume`, `Network` — sufficient to absorb the new layout's quadlet/ vs systemd/ split. The split is a source-side organization decision; the in-memory `quadlet_type` carries the runtime semantics.

### New types — **added in this feature**

The following are the only NEW types introduced in `src/core/`:

#### `LayoutError` (new, in `src/core/errors.rs`)

```rust
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum LayoutError {
    #[error("legacy layout artifact: {path}")]
    #[diagnostic(help(
        "see specs/016-source-repository-layout/contracts/layout.md and run \
         scripts/migrate-legacy-source-repo.sh"
    ))]
    LegacyArtifact { path: std::path::PathBuf },

    #[error("reserved name '{name}' (must not begin with '_' or '.')")]
    ReservedName { name: String },

    #[error("service '{service}' selected by host '{host}' has no directory under services/")]
    MissingService {
        host: String,
        service: String,
        #[source_code]
        src: miette::NamedSource<String>,
        #[label("declared here")]
        span: miette::SourceSpan,
    },

    #[error("malformed service.yaml: unknown key '{key}'")]
    UnknownServiceManifestKey {
        key: String,
        #[source_code]
        src: miette::NamedSource<String>,
        #[label]
        span: miette::SourceSpan,
    },

    #[error("malformed service.yaml: {message}")]
    ServiceManifestParse {
        message: String,
        #[source_code]
        src: miette::NamedSource<String>,
        #[label]
        span: miette::SourceSpan,
    },

    #[error("malformed host.yaml: {message}")]
    HostManifestParse {
        message: String,
        #[source_code]
        src: miette::NamedSource<String>,
        #[label]
        span: miette::SourceSpan,
    },

    #[error("config file destination escapes /etc/{config_root}/: {source_path}")]
    ConfigEscape {
        config_root: String,
        source_path: std::path::PathBuf,
    },

    #[error("destination conflict at {target}: {a} and {b}")]
    DestinationConflict {
        target: std::path::PathBuf,
        a: std::path::PathBuf,
        b: std::path::PathBuf,
    },

    #[error("host overlay introduces base unit at {path} (only drop-ins and config replacements allowed)")]
    HostOverlayBaseUnit { path: std::path::PathBuf },

    #[error("orphan drop-in at {path} (no matching unit '{unit}' in merged set)")]
    OrphanDropIn {
        path: std::path::PathBuf,
        unit: String,
    },
}
```

**Deliberate divergence**: the surrounding `errors.rs` module uses `thiserror` only. `LayoutError` adds `miette::Diagnostic` derivation because the parser context warrants source-span pointers (the offending YAML key, the `host.yaml` services list entry). Other errors in the module remain `thiserror`-only; this is an intentional asymmetry justified by the user-facing diagnostic quality requirement (FR-016, FR-017, FR-018).

#### `layout_version` field on `DesiredStateProvenance` (new, in `src/core/types.rs`)

The persisted controller status snapshot (`PersistedProvenanceState` → `DesiredStateProvenance`) gains one optional field:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesiredStateProvenance {
    pub repository: String,
    pub requested_ref: String,
    pub last_observed_revision: Option<String>,
    pub last_observed_at: Option<String>,

    /// Layout version of the source repository that produced this snapshot.
    /// "1" for the formalized layout introduced by spec 016. Absent on snapshots
    /// produced by pre-formalization controllers (treated as "0").
    #[serde(rename = "layout-version", default, skip_serializing_if = "Option::is_none")]
    pub layout_version: Option<String>,
}
```

The field is optional with a serde default to preserve backward-compatible reads of older snapshots (those produced by `core-ops` v1.x). v2.0.0 binaries always write `Some("1")`. The kebab-case JSON name `layout-version` is enforced via `#[serde(rename)]` per project YAML/JSON convention.

`PERSISTED_PROVENANCE_SCHEMA_VERSION` is **not** bumped: the field is additive and absent-field tolerance is provided by serde default. Existing v1 snapshots remain readable.

### Internal parser types — **inside `src/io/repo.rs`**

The following types live entirely inside the parser module and do not escape it. They are NOT added to `src/core/types.rs`. The parser walks the new layout, populates these internal types with span-tracked data, validates per FR-009 to FR-018, and finally lowers them to the existing `EvaluationInput { host, catalog, overlays }` shape that the planner consumes.

```rust
// All types below live in src/io/repo.rs (private, or pub(crate) only when
// needed for inline tests).

struct SourceRepository {
    services: BTreeMap<String, ParsedService>,
    hosts: BTreeMap<String, ParsedHost>,
    layout_version: u32,            // hard-coded to 1
}

struct ParsedService {
    id: String,                     // validated against reserved-name rule (FR-009)
    config_root: String,            // resolved from service.yaml or defaults to id
    manifest_present: bool,         // for diagnostic clarity
    quadlet: ParsedPayloadTree,     // empty if no quadlet/ dir
    systemd: ParsedPayloadTree,     // empty if no systemd/ dir
    config: ParsedConfigTree,       // empty if no config/ dir
}

struct ParsedHost {
    id: String,
    manifest: HostManifest,
    overlays: BTreeMap<String, ParsedHostOverlay>, // keyed by service id
}

struct ParsedHostOverlay {
    quadlet_dropins: BTreeMap<UnitName, Vec<ParsedDropIn>>,
    systemd_dropins: BTreeMap<UnitName, Vec<ParsedDropIn>>,
    config: ParsedConfigTree,
}

struct ParsedPayloadTree {
    kind: PayloadKind,                                  // Quadlet or Systemd
    units: BTreeMap<UnitName, ParsedUnit>,
    dropins: BTreeMap<UnitName, Vec<ParsedDropIn>>,
}

struct ParsedConfigTree {
    files: BTreeMap<RelativePath, ParsedConfigFile>,
}

struct UnitName { stem: String, extension: UnitExtension }
enum UnitExtension { Container, Volume, Network, Pod, Socket, Timer, Target, Mount, Path }
enum PayloadKind { Quadlet, Systemd }
struct RelativePath(std::path::PathBuf);                // validated for FR-010
struct ParsedUnit { source_path: PathBuf, contents: String }
struct ParsedDropIn { source_path: PathBuf, filename: String, contents: String }
struct ParsedConfigFile { source_path: PathBuf, contents: Vec<u8> }
```

#### Lowering to the planner contract

After validation completes, the parser converts `SourceRepository` → `EvaluationInput` for the host being planned:

| Internal | Lowered to |
|---|---|
| `ParsedService.quadlet.units[name]` | `ServiceDefinition.artifacts.push(ArtifactSource { quadlet_type, contents, source_path, ... })` |
| `ParsedService.systemd.units[name]` | same — `QuadletType` covers Socket/Timer/etc. |
| `ParsedService.{quadlet,systemd}.dropins[unit][file]` | `ServiceDefinition.base_dropins.push(DropInSource { target, contents, source_path })` where `target` is the resolved `/etc/.../<unit>.<ext>.d/<file>.conf` |
| `ParsedService.config.files[rel]` | `ServiceDefinition.config_files.push(ConfigFileSource { target_path: "/etc/<config-root>/<rel>", contents, source_path })` |
| `ParsedHost.overlays[svc].quadlet_dropins` and `.systemd_dropins` | merged in lex order (base-then-host) and emitted as `HostOverlaySet.overrides: Vec<DropInSource>` |
| `ParsedHost.overlays[svc].config.files[rel]` | `HostOverlaySet.config_overrides.push(ConfigFileSource { target_path, contents, source_path })` |

Per-service `config_root` does **not** need to be carried explicitly into `ServiceDefinition` because every config file's resolved `target_path` already encodes it. Operations that need the root (e.g. computing `managed_config_roots: Vec<String>` on `DesiredState`) recover it from the leading `/etc/<root>/` prefix of the target paths — the same logic the legacy `managed_config_root()` function performs today.

---

## Validation rule cross-reference

Each rule in the spec maps to a `LayoutError` variant or to a parser-internal validation site:

| Spec FR | Enforced by | Surfaces as |
|---|---|---|
| FR-009 (reserved names) | parser walk: directory-name check before parse | `LayoutError::ReservedName` |
| FR-010 (config path traversal) | `RelativePath::parse` (internal) | `LayoutError::ConfigEscape` |
| FR-011 (destination conflict) | conflict map built during lowering | `LayoutError::DestinationConflict` |
| FR-012 (legacy rejection) | pre-walk check for `quadlets/`, `quadlet-overrides/`, `overrides/` | `LayoutError::LegacyArtifact` |
| FR-013 (orphan drop-in) | post-merge cross-check during lowering | `LayoutError::OrphanDropIn` |
| FR-016 (missing service) | `host.yaml.services` resolution against parsed services | `LayoutError::MissingService` |
| FR-017 (malformed service.yaml) | `serde_yaml` deserialization with span tracking | `LayoutError::ServiceManifestParse`, `UnknownServiceManifestKey` |
| FR-018 (host overlay base unit) | host overlay walk: any non-`.d/` file rejects | `LayoutError::HostOverlayBaseUnit` |

---

## State persistence

The persisted snapshot `/var/lib/core-ops/status.json` gains one optional field on `DesiredStateProvenance`:

```json
{
  "schema_version": 1,
  "controller": { ... },
  "desired_state": {
    "repository": "...",
    "requested_ref": "...",
    "last_observed_revision": "...",
    "last_observed_at": "...",
    "layout-version": "1"
  },
  "reconciliation": { ... }
}
```

`PERSISTED_PROVENANCE_SCHEMA_VERSION` stays at `1`. The field is read with a serde default; older snapshots that lack it deserialize cleanly.

---

## Backward compatibility

None for the source repository layout itself — this is a major bump. The parser refuses legacy layouts unconditionally, with a diagnostic pointing at `scripts/migrate-legacy-source-repo.sh`. The planner contract (`EvaluationInput` and friends) is preserved, so the apply/plan/explain consumer code does not need to change.
