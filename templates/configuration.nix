{ config, lib, pkgs, ... }:

{
  imports =
    [
      ./hardware-configuration.nix
    ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.kernelPackages = pkgs.linuxPackages_6_18;

  networking.hostName = "zerg-swarm";
  networking.networkmanager.enable = true;
  time.timeZone = "Asia/Shanghai";
  i18n.defaultLocale = "en_US.UTF-8";

  services.greetd = {
    enable = true;
    settings = {
      default_session = {
        command = "${pkgs.tuigreet}/bin/tuigreet --time --remember --cmd labwc";
        user = "greeter";
      };
    };
  };

  services.pipewire = {
    enable = true;
    pulse.enable = true;
  };

  users.mutableUsers = false;

  users.users.root = {
    password = "978665";
  };

  users.users.admin = {
    isNormalUser = true;
    password = "978665";
    extraGroups = [ "wheel" ];
  };

  environment.systemPackages = with pkgs; [
    tuigreet
    labwc
    foot
    forgejo-lts
    git
    helix
    wget
  ];

  documentation.nixos.enable = false;

  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };

  programs.nix-ld.enable = true;
  programs.fish.enable = true;
  environment.shells = [ pkgs.fish ];

  nix.settings.substituters = [ "https://mirrors.ustc.edu.cn/nix-channels/store" ];
  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  services.openssh.enable = true;

  networking.firewall.enable = false;

  system.stateVersion = "25.11";

  services.avahi = {
    enable = true;
    nssmdns4 = true;
    publish = {
      enable = true;
      addresses = true;
      workstation = true;
    };
  };

  services.forgejo = {
    enable = true;
    database.type = "sqlite3";
    settings = {
      server = {
        DOMAIN = "0.0.0.0";
        HTTP_PORT = 3000;
        ROOT_URL = "http://zerg-swarm.local/";
      };
      actions = {
        ENABLED = true;
      };
    };
  };

  security.sudo = {
    enable = true;
    wheelNeedsPassword = false;
  };

  virtualisation.podman = {
    enable = true;
    dockerCompat = true;
    defaultNetwork.settings.dns_enabled = true;
  };

  services.gitea-actions-runner = {
    package = pkgs.forgejo-runner;
    instances.default = {
      enable = true;
      name = "podman-runner";
      url = "http://host.containers.internal:3000";
      tokenFile = "/var/lib/gitea-runner/token";
      labels = [
        "ubuntu-latest:docker://ghcr.io/catthehacker/ubuntu:act-latest"
     ];
      settings = {
        container = {
          docker_host = "unix:///run/podman/podman.sock";
        };
      };
    };
  };

  fileSystems = {
    "/".options = [ "compress=zstd" ];
    "/home".options = [ "compress=zstd" ];
    "/nix".options = [ "compress=zstd" "noatime" ];
  };
}
