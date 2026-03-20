{
  description = "Cerebrate - Agent cluster orchestrator for OpenZerg";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        craneLib = crane.mkLib pkgs;
        inherit (pkgs) lib;

        # Include templates directory in source
        staticFilter = path: type:
          (craneLib.filterCargoSources path type) ||
          (lib.hasInfix "/templates/" path) ||
          (type == "directory" && lib.hasSuffix "templates" path);

        src = lib.cleanSourceWith {
          src = lib.cleanSource ./.;
          filter = staticFilter;
        };

        commonArgs = {
          inherit src;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.protobuf ];
          buildInputs = with pkgs; [ openssl sqlite ];
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "cerebrate-deps";
        });

        cerebrate = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "cerebrate";
          cargoExtraArgs = "--bin cerebrate";
          doCheck = false;
        });

      in
      {
        packages = {
          inherit cerebrate;
          default = cerebrate;
        };

        devShells.default = craneLib.devShell {
          inherit src;
          inputsFrom = [ cerebrate ];
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
            cargo-llvm-cov
          ];
          shellHook = ''
            export LLVM_COV="${pkgs.llvmPackages_19.llvm}/bin/llvm-cov"
            export LLVM_PROFDATA="${pkgs.llvmPackages_19.llvm}/bin/llvm-profdata"
          '';
        };
      }
    ) // {
      overlays.default = final: prev: {
        cerebrate = self.packages.${final.system}.cerebrate;
      };

      nixosModules.default = { config, lib, pkgs, ... }: {
        imports = [ ./modules/swarm.nix ];
        config = lib.mkIf config.services.cerebrate.enable {
          services.cerebrate.package = lib.mkDefault self.packages.${pkgs.system}.cerebrate;
        };
      };
    };
}