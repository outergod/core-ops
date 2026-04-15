# Contract: Verification Scenario Runner Changes

**Feature**: 015-controller-state-lifecycle

---

## `VerificationCoreOpsActionKind` — New Variant

Add `Init` to the enum in `src/core/verification_model.rs`:

```rust
#[serde(rename_all = "snake_case")]
pub enum VerificationCoreOpsActionKind {
    Apply,
    Explain,
    Init,    // NEW
    Plan,
    Status,
    Agent,
}
```

---

## `VerificationCoreOpsAction` — Field Optionality

`repository_source` and `revision` are currently required string fields. After this change, they are only meaningful for `Init` steps. Make them optional with defaults:

```rust
pub struct VerificationCoreOpsAction {
    pub action: VerificationCoreOpsActionKind,
    #[serde(default)]
    pub repository_source: String,   // meaningful for Init only
    #[serde(default)]
    pub revision: String,            // meaningful for Init only
    // ... other fields unchanged
}
```

---

## `render_coreops_action` — Updated Behavior

| Action kind | Before | After |
|---|---|---|
| `Apply`, `Plan` | Appends `--repo <path> --rev <ref>` | No repo/ref args; just `--quadlet-dir` etc. |
| `Explain` | Appends `--repo <path> --rev <ref>` | No repo/ref args |
| `Agent` | Optionally appends `--repo`/`--rev` | No repo/ref args |
| `Init` | Did not exist | Emits `core-ops init <repository> <ref> [--force]` |
| `Status` | No change | No change |

**`Init` rendering**:
```
sudo <binary> init <repository_source_resolved> <revision> [--force if action.force is set]
```

`--force` flag needs a corresponding `#[serde(default)] pub force: bool` on `VerificationCoreOpsAction`.

---

## Scenario Step Format — `init` Step

All existing scenarios gain one (or two) new steps using this format:

```yaml
- step_id: init
  step_type: coreops_action
  target: guest
  action:
    action: init
    repository_source: fixture
    revision: <tag-or-branch-name>
```

For scenarios where the tracked ref changes between applies:

```yaml
- step_id: init-upgrade
  step_type: coreops_action
  target: guest
  action:
    action: init
    repository_source: fixture
    revision: <new-tag-or-branch-name>
    force: true
```

---

## Scenarios Requiring Two `init` Steps

| Scenario | First init | Second init (--force) |
|---|---|---|
| `accepted-layered-upgrade-transition` | `demo-uat-v2` | `demo-uat-v3` |
| `accepted-mount-removal-ordering` | `demo-mount-remove-v1` | `demo-mount-remove-v2` |
| `accepted-config-change-restart` | `config-v1` | `config-v2` |

---

## Fixture Repo Compatibility

The `revision` values used in existing scenarios (`demo-uat-v2`, `demo-uat-v3`, `config-v1`, etc.) are already tag names in the fixture repositories. The `init` command requires a branch or tag name resolvable in the repo. No fixture repo changes are needed — the existing tag names are valid `init` targets.
