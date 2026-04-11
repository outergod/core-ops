# Research: Unify CI Validation And Release Publication

## Decision 1: Job graph structure

**Decision**: Sequential jobs — `ci` → `build` (matrix) → `release` (master-only).

**Rationale**: Sequential ordering ensures distribution artifacts are only produced after governance and tests pass. The master-only gate on `release` (via `if: github.ref == 'refs/heads/master' && github.event_name == 'push'`) is the sole publication boundary. PRs still receive workflow artifacts from the `build` job (FR-002), gated on `ci` passing.

**Alternatives considered**:
- Parallel `ci` + `build`: would produce artifacts even when tests fail; rejected.
- Single monolithic job: incompatible with matrix cross-compilation; rejected.

---

## Decision 2: Cross-compilation matrix placement

**Decision**: Move the cross-compilation matrix (x86_64 + aarch64) from `release-binary.yml` into a `build` job in `ci.yml`, preserving all existing packaging steps identically (service files, tarballs, SHA256SUMS, release-metadata.json).

**Rationale**: `release-binary.yml` triggered on `release` event, creating a circular dependency (must create a release to build release binaries). Moving the build into `ci.yml` as an artifact-producing step breaks this cycle. Artifact names (`core-ops-binary-release-<target>`) are preserved to keep test assertions stable.

**Alternatives considered**:
- Reuse `release-binary.yml` as-is and just add a release job there: conflicts with FR-007 (retire separate workflow); rejected.

---

## Decision 3: Artifact download pattern for release job

**Decision**: Use `actions/download-artifact@v4` with `merge-multiple: true`, downloading all artifacts to `dist/`.

**Rationale**: `merge-multiple: true` (introduced in v4) merges files from both matrix artifacts into a single directory. Overlapping shared files (`core-ops.service`, `LICENSE`, `CHANGELOG.md`, `README.md`) are identical across matrix targets and harmlessly overwrite each other. Architecture-specific files (`core-ops-linux-amd64`, `core-ops-linux-arm64`, tarballs, SHA256SUMS) have distinct names and do not collide.

**Alternatives considered**:
- Download to named subdirectories then manually copy: unnecessary complexity; rejected.

---

## Decision 4: Duplicate tag detection strategy

**Decision**: `git ls-remote --tags origin "refs/tags/${tag}"` — fails with an explicit error message if the tag exists.

**Rationale**: Operates against the remote directly; works in a shallow clone; no GitHub API token needed beyond the default GITHUB_TOKEN. Clear actionable message directs operators to either delete the orphaned tag or bump `Cargo.toml` version. Checked before `gh release create` to surface the error explicitly (per FR-005) rather than relying on GitHub API error responses.

**Alternatives considered**:
- `gh api repos/{owner}/{repo}/git/refs/tags/{tag}`: requires parsing API response; adds complexity; rejected.
- Rely on `gh release create` failure: GitHub's error message is less actionable; rejected (FR-005 requires explicit message).

---

## Decision 5: CHANGELOG extraction for release notes

**Decision**: Parse `CHANGELOG.md` with `awk` to extract the block between `## [<version>]` and the next `## [` heading. Write to a temp file, pass via `--notes-file` to `gh release create`.

**Rationale**: Keep A Changelog format is already in use; awk extraction is dependency-free, reproducible, and produces the exact notes the maintainer wrote. `--notes-file` handles multi-line content without shell quoting issues.

**Alternatives considered**:
- `gh release create --generate-notes`: produces commit-log notes, not the maintained changelog; rejected (FR-011).
- `--notes "..."` with inline heredoc: shell quoting complexity; rejected.

---

## Decision 6: `release_identity` in `release-metadata.json`

**Decision**: Derive from `grep '^version' Cargo.toml` → `v${version}` format (e.g., `v0.7.0`).

**Rationale**: Cargo.toml is the single source of truth for version (per Principle 12). The `v` prefix aligns with the release tag format used in GitHub Releases and git tags. Previously derived from `github.event.release.tag_name` — replaced because the build job runs before the release exists.

**Alternatives considered**:
- `$GITHUB_REF_NAME`: varies by trigger context (branch name on non-release pushes); rejected.

---

## Decision 7: README badge service

**Decision**: GitHub-native workflow badge URLs for CI and E2E status; shields.io `github/v/release` badge for latest release version.

**Rationale**: GitHub's own `actions/workflows/<name>.yml/badge.svg` URLs are first-party, require no external service, and update in real time. The shields.io `github/v/release/<owner>/<repo>` badge is the de-facto standard for displaying latest GitHub Release version with live data.

**Alternatives considered**:
- shields.io for CI/E2E status too: introduces unnecessary external dependency for signals GitHub already serves natively; rejected.
- Badgen or other badge services: no material advantage; rejected.

---

## Decision 8: Release job permissions

**Decision**: Default `GITHUB_TOKEN` with `contents: write` scoped to the `release` job only. All other jobs retain `contents: read`.

**Rationale**: Per clarification Q1: no PAT or long-lived secret required. `contents: write` is sufficient for `git tag` + `gh release create` + asset upload via the GitHub API. Scoping to the release job minimizes the blast radius of the elevated permission.

**Alternatives considered**:
- Repository-level PAT: long-lived secret, broader scope, unnecessary risk; rejected.
- Workflow-level `contents: write`: elevates permission on PR runs unnecessarily; rejected.
