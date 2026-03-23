{
  description = "Project dev shell with recent Codex";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    codex.url = "github:openai/codex/rust-v0.89.0";
  };

  outputs = { self, nixpkgs, codex, ... }:
    let
      system = "x86_64-linux"; # change as needed
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          codex.packages.${system}.default
          yaml-language-server
          cargo
          rustc
          rust-analyzer
          rustfmt
          clippy
          pkg-config
          git
          uv
          python314
          podman
          systemd
          jq
        ];

        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    };
}
