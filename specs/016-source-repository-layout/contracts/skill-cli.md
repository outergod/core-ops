# Contract: `core-ops skill install`

**Feature**: 016-source-repository-layout
**Subcommand**: `core-ops skill install`

This document specifies the externally observable behavior of the `skill` subcommand introduced by spec 016. The contract is binding: tests assert on this surface.

## Synopsis

```text
core-ops skill install [--global] [--print]
```

`--global` and `--print` are mutually exclusive. The default mode (no flags) writes to the current working directory.

## Modes

### Default

- **Action**: writes the skill bundle to `<cwd>/.agents/skills/core-ops-source-repo/`.
- **Side effects**: creates the `.agents/skills/core-ops-source-repo/` directory tree if it does not exist; writes one file per bundle entry.
- **Idempotency**: re-running against an already-installed bundle either writes byte-identical files (no observable change) or refuses with a clear diagnostic if the existing tree is incompatible. Never produces a half-installed state.
- **Exit code**: 0 on success; non-zero on error. Errors are surfaced as `miette` diagnostics.

### `--global`

- **Action**: writes the skill bundle to `~/.agents/skills/core-ops-source-repo/`.
- All other behavior identical to default mode.

### `--print`

- **Action**: writes the skill bundle to standard output and performs **no** filesystem writes.
- **Format**: a concatenated stream where each bundle entry is preceded by a header line `==> <relative-path> <==` followed by the entry's bytes and a trailing newline. (Subject to one of two equivalent encodings — see "Bundle stream format" below.)
- **Use**: shell pipelines that filter or extract entries; CI sanity checks; comparison with the default-mode output.

## Bundle stream format (`--print`)

The print format MUST be reproducible: the same binary against the same inputs MUST produce byte-identical output.

```text
==> SKILL.md <==
<bytes of SKILL.md>
```

Multiple entries (when assets exist) are concatenated in lex order of relative path:

```text
==> SKILL.md <==
<bytes>
==> assets/<file> <==
<bytes>
```

If a future revision uses a tar stream instead, that is a backward-compatible change for consumers that use the file-based modes; the print format is documented as text-concatenation for v1.

## Path standard

The default and `--global` modes write to `.agents/skills/<skill-name>/` per the **agentskills.io** standard. The `<skill-name>` for this bundle is exactly `core-ops-source-repo`.

The binary MUST NOT default to vendor-specific paths such as `.claude/skills/` or `~/.claude/skills/`. A user who wants the bundle at a vendor-specific path can pipe `--print` to `tar` or copy the directory.

## Independence from `core-ops init`

The `skill install` subcommand MUST NOT:

- modify any file outside the resolved skill destination,
- consult `.specify/feature.json` or any controller state file,
- be invoked transitively by `core-ops init`,
- trigger any side effect on the controller's runtime state.

`init` initializes a controller; `skill install` installs an authoring aid. They share no state.

## Errors

| Condition | Diagnostic |
|---|---|
| `--global` and `--print` both set | clap-level argument conflict (handled by `clap` `conflicts_with`) |
| Destination directory not writable | `miette` error with the destination path and the underlying I/O error |
| Existing destination tree contains a file with the same path but different bytes | `miette` error naming the offending path; user must remove or back up the existing tree |
| `$HOME` undefined when `--global` is used | `miette` error explaining the requirement |

## Test contract

- **`test_skill_install_default`**: running `skill install` in a temp dir creates `.agents/skills/core-ops-source-repo/SKILL.md` with the embedded bundle's bytes.
- **`test_skill_install_global`**: with a temp `$HOME`, running `skill install --global` creates `$HOME/.agents/skills/core-ops-source-repo/SKILL.md` byte-identical to the default-mode output.
- **`test_skill_install_print`**: running `skill install --print` produces stdout containing every bundle entry, byte-identical concatenation matches a re-read of the on-disk default-mode output.
- **`test_skill_install_idempotent`**: running `skill install` twice in the same temp dir is a no-op on the second run (no error, no changed bytes).
- **`test_skill_install_no_init_coupling`**: `skill install` succeeds in a directory that is NOT a CoreOps source repository (no `services/`, no `hosts/`, no `.specify/`).
- **`test_skill_install_vendor_neutral`**: the destination path under `--global` MUST contain `.agents/skills/` and MUST NOT contain `.claude/skills/`.
