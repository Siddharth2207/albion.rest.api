{ pkgs, ragenix, rainix, system }:

let
  buildInputs =
    [ pkgs.terraform pkgs.rage pkgs.jq ragenix.packages.${system}.default ];

  tfState = "infra/terraform.tfstate";
  tfVars = "infra/terraform.tfvars";
  tfPlanFile = "infra/tfplan";

  parseIdentity = ''
    set -eo pipefail

    identity=~/.ssh/id_ed25519
    if [ "''${1:-}" = "-i" ]; then
      identity="$2"
      shift 2
    fi
  '';

  decryptState = ''
    if [ -f ${tfState}.age ]; then
      rage -d -i "$identity" ${tfState}.age > ${tfState}
    fi
  '';

  encryptState = ''
    if [ -f ${tfState} ]; then
      nix eval --raw --file ${
        ../keys.nix
      } roles.ssh --apply 'builtins.concatStringsSep "\n"' \
        | rage -e -R /dev/stdin -o ${tfState}.age ${tfState}
    fi
  '';

  cleanup = "rm -f ${tfState} ${tfState}.backup ${tfVars}";
  cleanupWithPlan = "${cleanup} ${tfPlanFile}";

  preamble = ''
    ${parseIdentity}
    on_exit() { ${cleanup}; }
    trap on_exit EXIT
    ${decryptVars}
  '';

  preambleWithEncrypt = ''
    ${parseIdentity}
    on_exit() {
      ${encryptState}
      ${cleanupWithPlan}
    }
    trap on_exit EXIT
    ${decryptVars}
  '';

  resolveIp = ''
    ${parseIdentity}
    trap 'rm -f ${tfState}' EXIT
    if [ -f ${tfState}.age ]; then
      if ! rage -d -i "$identity" ${tfState}.age > ${tfState}; then
        echo "Failed to decrypt ${tfState}.age with identity $identity" >&2
        exit 1
      fi
    elif [ ! -f ${tfState} ]; then
      echo "Neither ${tfState}.age nor ${tfState} exists; cannot resolve host IP" >&2
      exit 1
    fi

    host_ip=$(jq -r '.outputs.droplet_ipv4.value // empty' ${tfState} 2>/dev/null || true)
    if [ -z "$host_ip" ]; then
      outputs=$(jq -r '.outputs | keys | join(",")' ${tfState} 2>/dev/null || echo "<unreadable>")
      echo "Unable to find outputs.droplet_ipv4.value in terraform state (available outputs: $outputs)" >&2
      exit 1
    fi

    if ! echo "$host_ip" | grep -Eq '^([0-9]{1,3}\.){3}[0-9]{1,3}$'; then
      echo "Resolved host IP is not valid IPv4: '$host_ip'" >&2
      exit 1
    fi
  '';

  decryptVars = ''
    rage -d -i "$identity" ${tfVars}.age > ${tfVars}
  '';

  encryptVars = ''
    nix eval --raw --file ${
      ../keys.nix
    } roles.infra --apply 'builtins.concatStringsSep "\n"' \
      | rage -e -R /dev/stdin -o ${tfVars}.age ${tfVars}
  '';

  tfRekey = ''
    ${parseIdentity}
    on_exit() { ${cleanup}; }
    trap on_exit EXIT
    ${decryptState}
    ${encryptState}
    ${decryptVars}
    ${encryptVars}
  '';

in {
  inherit buildInputs parseIdentity resolveIp tfRekey;

  tfInit = rainix.mkTask.${system} {
    name = "tf-init";
    additionalBuildInputs = buildInputs;
    body = ''
      ${preamble}
      terraform -chdir=infra init "$@"
    '';
  };

  tfPlan = rainix.mkTask.${system} {
    name = "tf-plan";
    additionalBuildInputs = buildInputs;
    body = ''
      ${preamble}
      ${decryptState}
      terraform -chdir=infra plan -out=tfplan "$@"
    '';
  };

  tfApply = rainix.mkTask.${system} {
    name = "tf-apply";
    additionalBuildInputs = buildInputs;
    body = ''
      ${preambleWithEncrypt}
      ${decryptState}
      terraform -chdir=infra apply "$@" tfplan
    '';
  };

  tfDestroy = rainix.mkTask.${system} {
    name = "tf-destroy";
    additionalBuildInputs = buildInputs;
    body = ''
      ${preambleWithEncrypt}
      ${decryptState}
      terraform -chdir=infra destroy "$@"
    '';
  };

  tfEditVars = rainix.mkTask.${system} {
    name = "tf-edit-vars";
    additionalBuildInputs = buildInputs;
    body = ''
      ${parseIdentity}
      on_exit() { rm -f ${tfVars}; }
      trap on_exit EXIT

      if [ -f ${tfVars}.age ]; then
        ${decryptVars}
      else
        cp ${tfVars}.example ${tfVars}
      fi
      ''${EDITOR:-vi} ${tfVars}
      ${encryptVars}
    '';
  };
}
