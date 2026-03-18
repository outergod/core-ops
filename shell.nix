{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  strictDeps = true;

  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    rust-analyzer
    rustfmt
    clippy
    pkg-config
    git
    codex
    uv
    python314
  ];

  buildInputs = with pkgs; [
    podman
    systemd
    jq
  ];

  # Helpful for editors / rust-analyzer in Nix shells
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
}
