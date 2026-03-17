{
  description = "Zerg Swarm - Agent cluster manager for NixOS";

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
        src = lib.cleanSourceWith {
          src = lib.cleanSource ./.;
          filter = path: type:
            (type == "directory" && lib.hasSuffix "templates" path) ||
            (type == "regular" && lib.hasInfix "/templates/" path) ||
            (craneLib.filterCargoSources path type);
        };

        commonArgs = {
          inherit src;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [ openssl sqlite ];
        };

        cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
          pname = "zerg-swarm-deps";
        });

        zerg-swarm = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "zerg-swarm";
          cargoExtraArgs = "--bin zerg-swarm";
          doCheck = false;
        });

      in
      {
        packages = {
          inherit zerg-swarm;
          default = zerg-swarm;
        };

        devShells.default = craneLib.devShell {
          inherit src;
          inputsFrom = [ zerg-swarm ];
          packages = with pkgs; [
            rust-analyzer
            cargo-watch
            btrfs-progs
          ];
        };
      }
    ) // {
      overlays.default = final: prev: {
        zerg-swarm = self.packages.${final.system}.zerg-swarm;
      };

      nixosModules.default = { config, lib, pkgs, ... }: {
        imports = [ ./modules/swarm.nix ];
        config = lib.mkIf config.services.zerg-swarm.enable {
          services.zerg-swarm.package = lib.mkDefault self.packages.${pkgs.system}.zerg-swarm;
        };
      };
    };
}