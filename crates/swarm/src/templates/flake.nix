{
  description = "NixOS configuration for Zerg Swarm";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    zerg-swarm.url = "git+ssh://git@github.com/openzerg/zerg-swarm";
    openzerg.url = "git+ssh://git@github.com/openzerg/openzerg";
    zs-webui.url = "git+ssh://git@github.com/openzerg/zs-webui";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      zerg-swarm,
      openzerg,
      zs-webui,
      ...
    }:
    {
      nixosConfigurations = {
        zerg-swarm = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = {
            inherit inputs zerg-swarm openzerg;
          };
          modules = [
            ./configuration.nix
            ./generated/container.nix
            ./generated/filesystem.nix
            zerg-swarm.nixosModules.default
            ({ config, lib, pkgs, ... }: {
              services.zerg-swarm = {
                enable = true;
                port = 17531;
                dataDir = "/var/lib/zerg-swarm";
                username = "admin";
                password = "admin";
                package = zerg-swarm.packages.${pkgs.stdenv.hostPlatform.system}.zerg-swarm;
              };

              systemd.services.zs-webui = {
                description = "ZS WebUI";
                wantedBy = [ "multi-user.target" ];
                after = [ "network.target" ];
                requires = [ "zerg-swarm.service" ];

                serviceConfig = {
                  Type = "simple";
                  ExecStart = "${pkgs.serve}/bin/serve -s ${zs-webui.packages.${pkgs.stdenv.hostPlatform.system}.default} -l 8000";
                  Restart = "always";
                  RestartSec = "5s";
                };
              };

              networking.firewall.allowedTCPPorts = [ 17531 8000 ];

              environment.systemPackages = [
                zerg-swarm.packages.${pkgs.stdenv.hostPlatform.system}.zerg-swarm
              ];
            })
          ];
        };
      };
    };
}