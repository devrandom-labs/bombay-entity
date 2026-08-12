{
  description = "Bombay Entity — local entity runtime for Rust";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };
  outputs =
    {
      self,
      nixpkgs,
      utils,
      crane,
      fenix,
      advisory-db,
      ...
    }:
    utils.lib.eachSystem [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ] (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-mvUGEOHYJpn3ikC5hckneuGixaC+yGrkMM/liDIDgoU=";
        };
        miriToolchain = fenix.packages.${system}.combine [
          fenix.packages.${system}.latest.cargo
          fenix.packages.${system}.latest.rustc
          fenix.packages.${system}.latest.rust-src
          fenix.packages.${system}.latest.miri
        ];
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        miriCraneLib = (crane.mkLib pkgs).overrideToolchain miriToolchain;
        src = pkgs.lib.cleanSource ./.;
        commonArgs = {
          inherit src;
          pname = "bombay-entity";
          version = "0.1.0";
          strictDeps = true;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        miriVendorDir = miriCraneLib.vendorMultipleCargoDeps {
          cargoLockList = [
            ./Cargo.lock
            "${miriToolchain}/lib/rustlib/src/rust/library/Cargo.lock"
          ];
        };
        miriCheck = miriCraneLib.mkCargoDerivation (
          commonArgs
          // {
            cargoArtifacts = null;
            cargoVendorDir = miriVendorDir;
            pnameSuffix = "-miri";
            doInstallCargoArtifacts = false;
            buildPhaseCargoCommand = ''
              export MIRI_SYSROOT="$TMPDIR/miri-sysroot"
              cargo miri setup
              cargo miri test --workspace --lib --locked
            '';
          }
        );
      in
      {
        checks = {
          entity-build = craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; });
          entity-fmt = craneLib.cargoFmt { inherit src; };
          entity-clippy = craneLib.cargoClippy (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--workspace --all-targets -- --deny warnings";
            }
          );
          entity-nextest = craneLib.cargoNextest (commonArgs // { inherit cargoArtifacts; });
          entity-loom = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              RUSTFLAGS = "--cfg loom";
              cargoExtraArgs = "-p bombay-machine-executor --lib";
            }
          );
          entity-directory-loom = craneLib.cargoTest (
            commonArgs
            // {
              inherit cargoArtifacts;
              RUSTFLAGS = "--cfg loom";
              cargoExtraArgs = "-p bombay-entity --no-default-features";
              cargoTestExtraArgs = "--test loom_local_directory";
            }
          );
          entity-miri = miriCheck;
          entity-doctest = craneLib.cargoDocTest (commonArgs // { inherit cargoArtifacts; });
          entity-coverage = craneLib.cargoLlvmCov (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoLlvmCovCommand = "test";
              # Baseline measured 2026-08-12 at 93.22% lines (.tighten/BASELINE.md).
              cargoLlvmCovExtraArgs = "--workspace --fail-under-lines 93.2 --html --output-dir $out";
            }
          );
          entity-doc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });
          entity-audit = craneLib.cargoAudit { inherit src advisory-db; };
          entity-deny = craneLib.cargoDeny { inherit src; };
        };
        packages = {
          default = self.checks.${system}.entity-build;
          coverage = craneLib.cargoLlvmCov (
            commonArgs
            // {
              inherit cargoArtifacts;
              cargoLlvmCovCommand = "test";
              cargoLlvmCovExtraArgs = "--html --output-dir $out";
            }
          );
        };
        formatter = pkgs.nixfmt;
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = with pkgs; [
            cargo-audit
            cargo-deny
            cargo-llvm-cov
            cargo-nextest
            git
            gh
            just
            nixfmt
          ];
        };
      }
    );
}
