# Quickstart: Unify CI Validation And Release Publication

## Local Validation

### 1. Verify YAML syntax

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

No output = valid. (Requires `pyyaml`: `pip install pyyaml`.)

### 2. Run distribution integration tests

```bash
cargo test test_distribution_release
```

This runs:
- `release_workflow_includes_license_and_metadata_outputs` — verifies `ci.yml` packaging contract
- `unified_release_job_is_gated_to_master_push` — verifies release job structure
- `distribution_gate_is_split_between_public_ci_and_protected_e2e` — verifies `ci.yml` + `e2e-gate.yml` unchanged behavior

### 3. Run the full distribution test suite

```bash
cargo test test_distribution
```

### 4. Run clippy

```bash
cargo clippy --all-targets -- -D warnings
```

No new Rust source is introduced; this verifies no regressions in existing code.

---

## End-to-End Verification (requires GitHub access)

### PR validation (US1)

1. Open a PR from a branch with a compliant `changes/<id>.md` fragment.
2. Wait for CI to complete.
3. In the PR's **Checks** panel, open the `CI / Build Release Binaries` job.
4. Under **Artifacts**, confirm `core-ops-binary-release-x86_64-unknown-linux-gnu` and `core-ops-binary-release-aarch64-unknown-linux-gnu` are downloadable.

### Master push / release publication (US2)

1. Merge a PR that bumps `Cargo.toml` version and adds a `CHANGELOG.md` entry.
2. Wait for CI to complete on `master`.
3. Navigate to **Releases** on GitHub.
4. Confirm a release tagged `v<version>` exists with:
   - Binary assets attached (`core-ops-linux-amd64`, `.tar.gz`, `SHA256SUMS` for both arches)
   - Release body populated from the `## [<version>]` section of `CHANGELOG.md`

### Duplicate version guard (US2, SC-004)

1. Push a commit to `master` without bumping `Cargo.toml` version.
2. Confirm the `release` job fails with a message referencing the duplicate tag.

### Retired workflow (US3, SC-005)

```bash
ls .github/workflows/
# Expected: ci.yml  e2e-gate.yml  (no release-binary.yml)
```

### Live badges (US4, SC-006)

After at least one GitHub Release has been published:
1. View `README.md` on the `master` branch on GitHub.
2. Confirm the **Credibility** section shows live CI, E2E gate, and latest release badges (not static `0.7.0-dev`).

---

## Rollback / Recovery

**Orphaned tag**: If the `release` job pushes a tag but `gh release create` fails:
```bash
git push origin --delete v<version>
# Then re-trigger the release job (re-push or use Actions "Re-run jobs")
```
