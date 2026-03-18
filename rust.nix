{ pkgs, craneLib, sqlx-cli }:

let
  solcReleases = pkgs.fetchurl {
    url = "https://binaries.soliditylang.org/linux-amd64/list.json";
    sha256 = "sha256-L3zgoNUWLfEAFMNtPGTnIkn9+fCwRYNFlv7u8bysS9E=";
  };

  libDir = builtins.path {
    path = builtins.getEnv "PWD" + "/lib";
    name = "lib";
  };

  depsSrc = pkgs.lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      let base = builtins.baseNameOf path;
      in type == "directory" || base == "Cargo.toml" || base == "Cargo.lock";
  };

  cargoVendorDir = craneLib.vendorCargoDeps {
    src = ./.;
    cargoLock = ./Cargo.lock;
  };

  patchSvmBuildRs = ''
    while IFS= read -r -d "" file; do
      substituteInPlace "$file" \
        --replace "https://binaries.soliditylang.org/linux-amd64/list.json" "${solcReleases}"
    done < <(find . -path "*/svm-rs-builds-*/build.rs" -print0)
  '';

  commonArgs = {
    pname = "st0x-rest-api";
    version = "0.1.0";
    src = ./.;

    inherit cargoVendorDir;

    nativeBuildInputs = [ sqlx-cli pkgs.pkg-config pkgs.curl ];

    buildInputs = [ pkgs.openssl pkgs.sqlite ]
      ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin
      [ pkgs.apple-sdk_15 ];

    COMMIT_SHA = builtins.getEnv "COMMIT_SHA";

    postUnpack = ''
      rm -rf $sourceRoot/lib
      ln -s ${libDir} $sourceRoot/lib
    '';

    postPatch = patchSvmBuildRs;
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    src = depsSrc;
  });

  sqlxSetup = ''
    set -eo pipefail

    export DATABASE_URL="sqlite:$TMPDIR/build.db"
    sqlx db create
    sqlx migrate run --source migrations
  '';

in {
  package = craneLib.buildPackage (commonArgs // {
    inherit cargoArtifacts;
    preBuild = sqlxSetup;
    doCheck = true;

    meta = {
      description = "st0x REST API server";
      homepage = "https://github.com/ST0x-Technology/st0x-rest-api";
    };
  });

  clippy = craneLib.cargoClippy (commonArgs // {
    inherit cargoArtifacts;
    preBuild = sqlxSetup;
    cargoClippyExtraArgs = "--all-targets --all-features -- -D clippy::all";
  });
}