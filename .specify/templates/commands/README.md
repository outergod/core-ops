# Command Templates

This directory is reserved for Specify command templates.

Add `*.md` command templates here when you define custom commands for your
workflow. Keep them aligned with the project constitution, including
requirements for provenance, versioning, and machine-readable behavioral
traceability when commands introduce or validate runtime behavior. Commands that
change externally observable behavior, persisted schemas, CLI output,
reconciliation semantics, or compatibility must also update release-version
policy guidance, update the machine-checkable release-intent artifact, update
the Keep a Changelog-formatted changelog when the change is externally visible,
and preserve `Cargo.toml` as the canonical controller version.
Commands that complete Rust work should also enforce or reference the required
validation gates: `cargo test` and `cargo clippy --all-targets -- -D warnings`,
unless an explicit temporary exception is part of the workflow.
