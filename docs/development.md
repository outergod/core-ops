# Development (Nix + direnv)

This project assumes a Nix shell provided by `shell.nix` and loaded via direnv.
Do not assume Rust tooling is installed globally.

## Setup

1. Install direnv and Nix.
2. Run `direnv allow` at repo root.

## Common Commands

- Format: `cargo fmt` (or `make fmt`)
- Lint: `cargo clippy --all-targets --all-features -- -D warnings` (or `make lint`)
- Test: `cargo test` (or `make test`)

Assume the nix shell is already active, and do not run commands via `direnv exec`.
