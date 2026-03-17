{ config, lib, pkgs, ... }:

let
  cfg = config.services.zerg-swarm;
in
{
  options.services.zerg-swarm = {
    enable = lib.mkEnableOption "Zerg Swarm Manager";

    port = lib.mkOption {
      type = lib.types.port;
      default = 17531;
      description = "Port for API, WebSocket, and LLM proxy";
    };

    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/zerg-swarm";
      description = "Data directory for database and configuration";
    };

    username = lib.mkOption {
      type = lib.types.str;
      default = "admin";
      description = "Admin username for authentication";
    };

    password = lib.mkOption {
      type = lib.types.str;
      default = "admin";
      description = "Admin password for authentication";
    };

    package = lib.mkOption {
      type = lib.types.package;
      description = "The zerg-swarm package to use";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.zerg-swarm = {
      description = "Zerg Swarm Manager";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        ZERG_SWARM_PORT = toString cfg.port;
        ZERG_SWARM_USERNAME = cfg.username;
        ZERG_SWARM_PASSWORD = cfg.password;
        ZERG_SWARM_DATA_DIR = cfg.dataDir;
        RUST_LOG = "info";
      };

      path = [ pkgs.btrfs-progs ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/zerg-swarm serve";
        Restart = "always";
        RestartSec = "5s";
        WorkingDirectory = cfg.dataDir;
        StateDirectory = builtins.baseNameOf cfg.dataDir;
      };
    };

    networking.firewall.allowedTCPPorts = [ cfg.port ];
  };
}