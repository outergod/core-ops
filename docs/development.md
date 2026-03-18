# Development (Nix + direnv)

This project assumes a Nix shell provided by `shell.nix` and loaded via direnv.
Do not assume Rust tooling is installed globally.

## Setup

1. Install direnv and Nix.
2. Run `direnv allow` at repo root.

## Common Commands

- Format: `make fmt`
- Lint: `make lint`
- Test: `make test`

These commands run via `direnv exec .` to ensure the Nix shell is active.
