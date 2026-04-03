{ deploy-rs, self }:

let
  system = "x86_64-linux";
  inherit (deploy-rs.lib.${system}) activate;
  profileBase = "/nix/var/nix/profiles/per-service";

  albionPackage = self.packages.${system}.albion-rest-api;

  services = import ./services.nix;
  enabledServices = builtins.attrNames (builtins.removeAttrs services
    (builtins.filter (n: !services.${n}.enabled)
      (builtins.attrNames services)));

  mkServiceProfile = name:
    let
      markerFile = "/run/albion/${name}.ready";
    in activate.custom albionPackage (builtins.concatStringsSep " && " [
      "systemctl stop ${name} || true"
      "rm -f ${markerFile}"
      "mkdir -p /run/albion"
      "touch ${markerFile}"
      "systemctl restart ${name}"
    ]);

  mkProfile = name: {
    path = mkServiceProfile name;
    profilePath = "${profileBase}/${name}";
  };

in {
  config = {
    nodes.albion-rest-api = {
      hostname = builtins.getEnv "DEPLOY_HOST";
      sshUser = "root";
      user = "root";

      profilesOrder = [ "system" ] ++ enabledServices;

      profiles = {
        system.path = activate.nixos self.nixosConfigurations.albion-rest-api;
      } // builtins.listToAttrs (map (name: {
        inherit name;
        value = mkProfile name;
      }) enabledServices);
    };
  };

  wrappers = { pkgs, infraPkgs, localSystem }:
    let
      deployInputs = infraPkgs.buildInputs
        ++ [ deploy-rs.packages.${localSystem}.deploy-rs ];

      deployPreamble = ''
        ${infraPkgs.parseIdentity}
        if [ -n "''${DEPLOY_HOST:-}" ]; then
          host_ip="$DEPLOY_HOST"
        else
          trap 'rm -f infra/terraform.tfstate' EXIT
          if [ -f infra/terraform.tfstate.age ]; then
            if ! rage -d -i "$identity" infra/terraform.tfstate.age > infra/terraform.tfstate; then
              echo "Failed to decrypt infra/terraform.tfstate.age with identity $identity" >&2
              exit 1
            fi
          elif [ ! -f infra/terraform.tfstate ]; then
            echo "Neither infra/terraform.tfstate.age nor infra/terraform.tfstate exists; cannot resolve host IP" >&2
            exit 1
          fi

          host_ip=$(jq -r '.outputs.droplet_ipv4.value // empty' infra/terraform.tfstate 2>/dev/null || true)
          if [ -z "$host_ip" ]; then
            outputs=$(jq -r '.outputs | keys | join(\",\")' infra/terraform.tfstate 2>/dev/null || echo "<unreadable>")
            echo "Unable to find outputs.droplet_ipv4.value in terraform state (available outputs: $outputs)" >&2
            exit 1
          fi

          if ! echo "$host_ip" | grep -Eq '^([0-9]{1,3}\\.){3}[0-9]{1,3}$'; then
            echo "Resolved host IP is not valid IPv4: '$host_ip'" >&2
            exit 1
          fi

          export DEPLOY_HOST="$host_ip"
        fi
        export NIX_SSHOPTS="-i $identity"
        ssh_flag="--ssh-opts=-i $identity"
      '';

      deployFlags = if localSystem == "x86_64-linux" then
        ""
      else
        "--skip-checks --remote-build";

    in {
      deployNixos = pkgs.writeShellApplication {
        name = "deploy-nixos";
        runtimeInputs = deployInputs;
        text = ''
          ${deployPreamble}
          deploy ${deployFlags} ''${ssh_flag:+"$ssh_flag"} .#albion-rest-api.system \
            -- --impure "$@"
        '';
      };

      deployService = pkgs.writeShellApplication {
        name = "deploy-service";
        runtimeInputs = deployInputs;
        text = ''
          ${deployPreamble}
          profile="''${1:?usage: deploy-service <profile>}"
          shift
          deploy ${deployFlags} ''${ssh_flag:+"$ssh_flag"} ".#albion-rest-api.$profile" \
            -- --impure "$@"
        '';
      };

      deployAll = pkgs.writeShellApplication {
        name = "deploy-all";
        runtimeInputs = deployInputs;
        text = ''
          ${deployPreamble}
          deploy ${deployFlags} ''${ssh_flag:+"$ssh_flag"} .#albion-rest-api \
            -- --impure "$@"
        '';
      };
    };
}
