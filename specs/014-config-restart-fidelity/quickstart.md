# Quickstart: Config Change Restart Fidelity

## What changed

`core-ops apply` now correctly restarts containers whose config files change,
are removed, or are added (when the container was already running). Previously,
the apply report claimed the container `restarted` but no restart occurred.

## Trigger condition

A container is restarted when `apply` runs and ANY of the following is true:

- A `ConfigFile` it references via `EnvironmentFile=` has changed contents.
- A `ConfigFile` it references has been removed (restart surfaces the failure).
- A `ConfigFile` it references was added and the container was already running
  before this apply.

## Validation

```bash
# Before apply: note the start timestamp
systemctl show github-actions-runner.service -p ActiveEnterTimestamp

# Apply a config file change
core-ops apply --repo file:///var/lib/core-ops/repo --rev master

# After apply: timestamp should be newer
systemctl show github-actions-runner.service -p ActiveEnterTimestamp
```

## Regression test

```bash
cargo test config_file_change_schedules_restart
```

## Release governance

- Bump: `patch`
- Fragment: `changes/014-config-restart-fidelity.md`
