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

          # Channel + components come from rust-toolchain.toml; coverage needs llvm-tools on top. `.override
          # { extensions }` replaces the file's components rather than adding to them (rust-overlay
          # lib/rust-bin.nix `fromRustupToolchain` sets `extensions = components`), so re-list them here or
          # the shell loses rustfmt and clippy (#364).
          toolchainFile = (builtins.fromTOML (builtins.readFile ./rust-toolchain.toml)).toolchain;
          rustStable = (pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
            extensions = toolchainFile.components ++ [ "llvm-tools-preview" ];
          };
          # cargo-fuzz needs nightly (`-Zsanitizer`); used only by the fuzz shell. `selectLatestNightlyWith`
          # picks the newest nightly that has every listed component, so a `nix flake update` cannot land
          # on one with a component missing (oxalica/rust-overlay README).
          rustNightly = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal);

          # Not packaged in nixpkgs, or older there than CI tracks: built from crates.io, pinned.
          arch-lintSource = pkgs.rustPlatform.buildRustPackage rec {
            pname = "arch-lint-cli";
            version = "0.11.0";
            src = pkgs.fetchCrate { inherit pname version; hash = "sha256-AiIaDp+3wVcG3HHTrpJg0sV3HDqGky1wiocPMPLyCyE="; };
            cargoHash = "sha256-FHZHU1/Gocvm00/F1Nz/4muMGb2c1WaEB9zwshWNFQg=";
            doCheck = false;
          };
          # Prebuilt upstream release: static musl, so it runs unpatched. Upstream ships musl only for
          # x86_64-linux (aarch64-linux has only -gnu), and that is the CI platform, where the source build costs
          # about 1m49s cold. The hash is the release's `.sha256` asset (equal to GitHub's digest); update it with
          # `version` and arch-lintSource. ADR-0029.
          arch-lintPrebuilt = pkgs.stdenvNoCC.mkDerivation rec {
            pname = "arch-lint-cli";
            version = "0.11.0";
            src = pkgs.fetchurl {
              url = "https://github.com/ynishi/arch-lint/releases/download/arch-lint-v${version}/arch-lint-x86_64-unknown-linux-musl.tar.gz";
              hash = "sha256-nTKYE1SVCZXgpz9vZ/v2xDrvucLLrJ+DOIPCOdrkS9g=";
            };
            sourceRoot = ".";
            installPhase = "install -Dm755 -t $out/bin arch-lint cargo-arch-lint";
          };
          arch-lint-cli = if system == "x86_64-linux" then arch-lintPrebuilt else arch-lintSource;
          # Prebuilt upstream release (static musl on Linux): compiling rumdl from source takes ~10 minutes
          # and ~18 GB of RAM, which timed out the cold Markdown CI job. Hashes are the release's digests.
          # To bump: set `version`, then for each of the three triples below run
          #   nix store prefetch-file --hash-type sha256 <release-url>
          # which prints the SRI hash `fetchurl` needs (GitHub's release digests are hex, not SRI). ADR-0029.
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

          # mold is the mandatory Linux linker (issue #153): `.cargo/config.toml` links through
          # `clang -fuse-ld=mold`, so a Linux build outside the flake fails with a clear linker
          # error. Same shells as `nativeLibs`: exactly the ones that link a binary.
          linuxLinker = lib.optionals stdenv.hostPlatform.isLinux [
            pkgs.clang
            pkgs.mold
          ];

          # One shell per CI job family, so a job pays only for the tools it uses.
          mkShell =
            {
              name,
              packages,
              native ? false,
            }:
            pkgs.mkShell {
              inherit name;
              packages = packages ++ lib.optionals native ([ pkgs.pkg-config ] ++ nativeLibs ++ linuxLinker);
              ALLOY_DEV_SHELL = name;
              # Only shells that build or run the workspace pull the wayland/X11 closure.
              LD_LIBRARY_PATH = lib.optionalString (native && stdenv.hostPlatform.isLinux) (
                lib.makeLibraryPath nativeLibs
              );
            };

          # ci/*.sh read JSON with jq; the flake supplies it instead of the host.
          scriptTools = [ pkgs.jq ];

          # Package lists of the six shells that share the `toolchain` cache entry; the `toolchain` shell is
          # built from them, so it cannot drift from the shells it stands in for (#357).
          rustPackages = [ rustStable ] ++ scriptTools;
          coveragePackages = [ rustStable pkgs.cargo-llvm-cov ];
          unsafeAuditPackages = [ rustStable pkgs.cargo-geiger ] ++ scriptTools;
          fuzzPackages = [ rustNightly pkgs.cargo-fuzz ];
          supplyChainPackages = [ rustStable pkgs.cargo-deny ];
          archPackages = [ rustStable arch-lint-cli ];

          # `tools` reads and writes GitHub through the gh CLI.
          repoTools = [
            tools.packages.${system}.default
            pkgs.gh
            pkgs.just
          ];
        in
        {
          markdown = mkShell { name = "markdown"; packages = [ rumdl ]; };
          rust = mkShell { name = "rust"; packages = rustPackages; native = true; };
          supply-chain = mkShell { name = "supply-chain"; packages = supplyChainPackages; };
          unsafe-audit = mkShell {
            name = "unsafe-audit";
            packages = unsafeAuditPackages;
            native = true;
          };
          # arch-lint reads cargo metadata.
          arch = mkShell { name = "arch"; packages = archPackages; };
          coverage = mkShell { name = "coverage"; packages = coveragePackages; native = true; };
          # `just issues-check` / `repo-check` (needs a token and the network, so not in `just gate`).
          repo = mkShell { name = "repo"; packages = repoTools; };
          fuzz = mkShell { name = "fuzz"; packages = fuzzPackages; native = true; };
          # Never entered by a job's commands: the nix-shell action's rooting step (`--profile`) realises and roots
          # it in the CI job that saves the shared `toolchain` Nix cache entry, so the entry holds the union of the
          # rust, coverage, unsafe-audit, fuzz, supply-chain and arch closures (#357, ADR-0029).
          toolchain = mkShell {
            name = "toolchain";
            packages = lib.unique (
              rustPackages
              ++ coveragePackages
              ++ unsafeAuditPackages
              ++ fuzzPackages
              ++ supplyChainPackages
              ++ archPackages
            );
            native = true;
          };

          default = mkShell {
            name = "default";
            packages = [
              rustStable
              pkgs.lefthook
              pkgs.cargo-deny
              pkgs.cargo-llvm-cov
              pkgs.cargo-geiger
              # Optional `rustc` wrapper (issue #151): opt in with `export RUSTC_WRAPPER=sccache`.
              pkgs.sccache
              arch-lint-cli
              rumdl
            ] ++ repoTools ++ scriptTools;
            native = true;
          };
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt);
    };
}
