{ pkgs, lib, modulesPath, docsRoot, ... }:

let
  inherit (import ./keys.nix) roles;

  services = import ./services.nix;
  enabledServices = lib.filterAttrs (_: v: v.enabled) services;

  mkService = name: cfg:
    let
      path = "/nix/var/nix/profiles/per-service/${name}/bin/${cfg.bin}";
      configFile = ./config/${name}.toml;
    in {
      description = "Albion ${cfg.bin} (${name})";

      wantedBy = [ ];

      restartIfChanged = false;
      stopIfChanged = false;

      unitConfig = {
        "X-OnlyManualStart" = true;
        ConditionPathExists = "/run/albion/${name}.ready";
      };

      serviceConfig = {
        DynamicUser = true;
        SupplementaryGroups = [ "albion" ];
        ExecStart = "${path} serve --config ${configFile}";
        Restart = "always";
        RestartSec = 5;
        ReadWritePaths = [ "/mnt/data" ];
      };
    };

in {
  imports = [
    (modulesPath + "/virtualisation/digital-ocean-config.nix")
    (modulesPath + "/profiles/qemu-guest.nix")
    ./disko.nix
  ];

  boot.loader.grub = {
    efiSupport = true;
    efiInstallAsRemovable = true;
  };

  networking.useDHCP = lib.mkForce false;

  services = {
    cloud-init = {
      enable = true;
      network.enable = true;
      settings = {
        datasource_list = [ "ConfigDrive" "Digitalocean" ];
        datasource.ConfigDrive = { };
        datasource.Digitalocean = { };
        cloud_init_modules = [
          "seed_random"
          "bootcmd"
          "write_files"
          "growpart"
          "resizefs"
          "set_hostname"
          "update_hostname"
          "set_password"
        ];
        cloud_config_modules =
          [ "ssh-import-id" "keyboard" "runcmd" "disable_ec2_metadata" ];
        cloud_final_modules = [
          "write_files_deferred"
          "puppet"
          "chef"
          "ansible"
          "mcollective"
          "salt_minion"
          "reset_rmc"
          "scripts_per_once"
          "scripts_per_boot"
          "scripts_user"
          "ssh_authkey_fingerprints"
          "keys_to_console"
          "install_hotplug"
          "phone_home"
          "final_message"
        ];
      };
    };

    openssh = {
      enable = true;
      settings = {
        PasswordAuthentication = false;
        PermitRootLogin = "prohibit-password";
      };
    };

    nginx = {
      enable = true;
      recommendedTlsSettings = true;
      recommendedProxySettings = true;
      virtualHosts."api.albion.rest" = {
        enableACME = true;
        forceSSL = true;
        locations."/" = {
          proxyPass = "http://127.0.0.1:8000";
        };
      };
    };
  };

  users.users.root.openssh.authorizedKeys.keys = roles.ssh;

  security.acme = {
    acceptTerms = true;
    defaults.email = "ops@albion.rest";
  };

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [
      22
      80
      443
    ];
  };

  fileSystems."/mnt/data" = {
    device = "/dev/disk/by-id/scsi-0DO_Volume_albion-rest-api-data";
    fsType = "ext4";
  };

  nix = {
    settings = {
      experimental-features = [ "nix-command" "flakes" ];
      auto-optimise-store = true;
      download-buffer-size = 268435456;
      sandbox = "relaxed";
    };

    gc = {
      automatic = true;
      dates = "weekly";
      options = "--delete-older-than 30d";
    };
  };

  users.groups.albion = { };
  programs.bash.interactiveShellInit = "set -o vi";

  services.logrotate = {
    enable = true;
    settings."/mnt/data/albion-rest-api/logs/*.log" = {
      su = "root albion";
      rotate = 14;
      weekly = true;
      compress = true;
      missingok = true;
      notifempty = true;
    };
  };

  systemd.tmpfiles.rules = [
    "d /mnt/data/albion-rest-api 0775 root albion -"
    "d /mnt/data/albion-rest-api/logs 0775 root albion -"
  ];
  systemd.services = lib.mapAttrs mkService enabledServices;

  environment.systemPackages = with pkgs; [
    bat
    curl
    htop
    sqlite
    zellij
  ];

  system.activationScripts.per-service-profiles.text =
    "mkdir -p /nix/var/nix/profiles/per-service";

  system.activationScripts.albion-docs.text = ''
    ln -sfn ${docsRoot} /var/lib/albion-docs
  '';

  swapDevices = [{
    device = "/swapfile";
    size = 4096;
  }];

  system.stateVersion = "24.11";
}
