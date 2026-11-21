# Required development environment (ADR-0029, issue #152): supersedes ADR-0025 decisions 1 and 4.
#
# `nix develop` (or direnv's `use flake`) is how Alloy is built, tested and run on
# Linux and macOS. The Rust channel is NOT declared here: it is read from
# `rust-toolchain.toml`, so rustup users, IDEs and the Windows CI job agree.
{
  description = "Alloy development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
          inherit (pkgs) lib stdenv;

          # Channel + rustfmt/clippy come from rust-toolchain.toml; coverage needs llvm-tools.
          rustStable = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
            extensions = [ "llvm-tools-preview" ];
          };
          # cargo-fuzz needs nightly (`-Zsanitizer`); used only by the fuzz shell.
          rustNightly = pkgs.rust-bin.nightly.latest.minimal;

          # Not packaged in nixpkgs, or older there than CI tracks: built from crates.io, pinned.
          arch-lint-cli = pkgs.rustPlatform.buildRustPackage rec {
            pname = "arch-lint-cli";
            version = "0.11.0";
            src = pkgs.fetchCrate { inherit pname version; hash = "sha256-AiIaDp+3wVcG3HHTrpJg0sV3HDqGky1wiocPMPLyCyE="; };
            cargoHash = "sha256-FHZHU1/Gocvm00/F1Nz/4muMGb2c1WaEB9zwshWNFQg=";
            doCheck = false;
          };
          rumdl = pkgs.rustPlatform.buildRustPackage rec {
            pname = "rumdl";
            version = "0.3.5";
            src = pkgs.fetchCrate { inherit pname version; hash = "sha256-MO3zK3rhydjLGLCkD/1XZcmqxsvScgrQ214Qh3FMPyw="; };
            cargoHash = "sha256-PFNjs6jAnVjbJo97CA42OKBVxVAL/r1tK+jF+pcOtcw=";
            # nixpkgs exports JEMALLOC_OVERRIDE to a path that does not exist; build the vendored jemalloc.
            preBuild = "unset JEMALLOC_OVERRIDE";
            doCheck = false;
          };

          # winit + softbuffer link or dlopen these at run time on Linux.
          nativeLibs = lib.optionals stdenv.hostPlatform.isLinux (
            with pkgs;
            [
              zlib # rust-overlay's clippy-driver dlopens libz
              wayland
              libxkbcommon
              libx11
              libxcursor
              libxi
              libxrandr
            ]
          );

          # One shell per CI job family, so a job pays only for the tools it uses.
          mkShell =
            {
              name,
              packages,
              native ? false,
            }:
            pkgs.mkShell {
              inherit name;
              packages = packages ++ lib.optionals native ([ pkgs.pkg-config ] ++ nativeLibs);
              ALLOY_DEV_SHELL = name;
              # Only shells that build or run the workspace pull the wayland/X11 closure.
              LD_LIBRARY_PATH = lib.optionalString (native && stdenv.hostPlatform.isLinux) (
                lib.makeLibraryPath nativeLibs
              );
            };

          # ci/*.sh read JSON with jq; the flake supplies it instead of the host.
          scriptTools = [ pkgs.jq ];
        in
        {
          markdown = mkShell { name = "markdown"; packages = [ rumdl ]; };
          rust = mkShell { name = "rust"; packages = [ rustStable ] ++ scriptTools; native = true; };
          supply-chain = mkShell { name = "supply-chain"; packages = [ rustStable pkgs.cargo-deny ]; };
          unsafe-audit = mkShell {
            name = "unsafe-audit";
            packages = [ rustStable pkgs.cargo-geiger ] ++ scriptTools;
            native = true;
          };
          # arch-lint reads cargo metadata.
          arch = mkShell { name = "arch"; packages = [ rustStable arch-lint-cli ]; };
          coverage = mkShell { name = "coverage"; packages = [ rustStable pkgs.cargo-llvm-cov ]; native = true; };
          fuzz = mkShell { name = "fuzz"; packages = [ rustNightly pkgs.cargo-fuzz ]; native = true; };

          default = mkShell {
            name = "default";
            packages = [
              rustStable
              pkgs.just
              pkgs.lefthook
              pkgs.cargo-deny
              pkgs.cargo-llvm-cov
              pkgs.cargo-geiger
              arch-lint-cli
              rumdl
            ] ++ scriptTools;
            native = true;
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
