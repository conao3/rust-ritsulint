{
  description = "A linter for Japanese statutes, with its rules for reading provisions written in Lean 4 and compiled to Rust";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    leanfran = {
      url = "github:conao3/lean-leanfran/420a7dce66dda3cbfd03b884d453cb8de63a0fc6";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, nixpkgs, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];

      imports = [ inputs.treefmt-nix.flakeModule ];

      perSystem =
        { system, ... }:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };
          rust = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "clippy"
              "rust-src"
              "rustfmt"
            ];
          };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
          src = pkgs.lib.cleanSource ./.;
        in
        {
          _module.args.pkgs = pkgs;

          packages.default = rustPlatform.buildRustPackage {
            pname = "ritsulint";
            version = "0.1.0";
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
            meta = {
              description = "A linter for Japanese statutes";
              mainProgram = "ritsulint";
            };
          };

          devShells.default = pkgs.mkShell {
            packages = [
              rust
              pkgs.lean4
              inputs.leanfran.packages.${system}.leanfran
              pkgs.gnumake
              pkgs.jq
              pkgs.nodejs_26
              pkgs.pnpm
            ];
          };

          treefmt = {
            programs.nixfmt.enable = true;
            programs.rustfmt = {
              enable = true;
              package = rust;
              edition = "2024";
            };
            settings.formatter.rustfmt.excludes = [ "src/generated/**" ];
          };
        };
    };
}
