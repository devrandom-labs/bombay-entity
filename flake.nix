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
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        src = pkgs.lib.cleanSource ./.;
        commonArgs = {
          inherit src;
          pname = "bombay-entity";
          version = "0.1.0";
          strictDeps = true;
        };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
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
          entity-doctest = craneLib.cargoDocTest (commonArgs // { inherit cargoArtifacts; });
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
