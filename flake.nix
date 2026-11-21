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
    # Declares and checks labels, milestones and repo settings (tools.toml, issue #334). The tag
    # is the only version written here; flake.lock pins the revision.
    tools = {
      url = "github:jaoppb/tools/v0.3.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      tools,
      ...
    }:
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
            version = "0.6.0";
            src = pkgs.fetchCrate { inherit pname version; hash = "sha256-Ws+QV5nKM8fOV9zW5GvgePy93NQaOrcbtF2EETfK9Cc="; };
            cargoHash = "sha256-CfrH3ljolTef/BP4kOanlz1qSWRUtbeEbA+TUfTWSnU=";
            doCheck = false;
          };
          # Prebuilt upstream release (static musl on Linux): compiling rumdl from source takes ~10 minutes
          # and ~18 GB of RAM, which timed out the cold Markdown CI job. Hashes are the release's digests.
          rumdl =
            let
              version = "0.3.5";
              target =
                {
                  x86_64-linux = {
                    triple = "x86_64-unknown-linux-musl";
                    hash = "sha256-R6y6dwb/eleqOOys3llT2s1BH+yk024tbJlLGjMlPsQ=";
                  };
                  aarch64-linux = {
                    triple = "aarch64-unknown-linux-musl";
                    hash = "sha256-H2uBmM5oj299uMm64uoK4TuTr0RUc+FsYORXmAkSGTE=";
                  };
                  x86_64-darwin = {
                    triple = "x86_64-apple-darwin";
                    hash = "sha256-p/Sv6/gHmzh+/HquWu3eL50idOl2LU96CZ1wG1AFwII=";
                  };
                  aarch64-darwin = {
                    triple = "aarch64-apple-darwin";
                    hash = "sha256-fK62wCZJcEecDAGFyTZGMlW3tgSOZ1pkffvrqzN/kf4=";
                  };
                }
                .${system};
            in
            pkgs.stdenvNoCC.mkDerivation {
              pname = "rumdl";
              inherit version;
              src = pkgs.fetchurl {
                url = "https://github.com/rvben/rumdl/releases/download/v${version}/rumdl-v${version}-${target.triple}.tar.gz";
                inherit (target) hash;
              };
              sourceRoot = ".";
              installPhase = "install -Dm755 rumdl $out/bin/rumdl";
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

          # `tools` reads and writes GitHub through the gh CLI.
          repoTools = [
            tools.packages.${system}.default
            pkgs.gh
            pkgs.just
          ];
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
          # `just issues-check` / `repo-check` (needs a token and the network, so not in `just gate`).
          repo = mkShell { name = "repo"; packages = repoTools; };
          fuzz = mkShell { name = "fuzz"; packages = [ rustNightly pkgs.cargo-fuzz ]; native = true; };

          default = mkShell {
            name = "default";
            packages = [
              rustStable
              pkgs.lefthook
              pkgs.cargo-deny
              pkgs.cargo-llvm-cov
              pkgs.cargo-geiger
              arch-lint-cli
              rumdl
            ] ++ repoTools ++ scriptTools;
            native = true;
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
    };
}
