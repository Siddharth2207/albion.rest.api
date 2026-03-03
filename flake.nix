{
  description = "Dev shell (Rust 1.91+ + ragenix)";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    rainix.url = "github:rainprotocol/rainix";
    ragenix.url = "github:yaxitech/ragenix";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "rainix/nixpkgs";
  };

  outputs = { self, flake-utils, rainix, ragenix, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = rainix.pkgs.${system};

        # ✅ Use stable Rust from fenix (will be >= 1.91 if stable is >= 1.91)
        rustToolchain = fenix.packages.${system}.stable.toolchain;
      in {
        devShells.default = pkgs.mkShell {
          inherit (rainix.devShells.${system}.default) nativeBuildInputs;

          buildInputs =
            [
              rustToolchain
              pkgs.rust-analyzer
              ragenix.packages.${system}.default
            ]
            ++ (rainix.devShells.${system}.default.buildInputs or []);
        };
      });
}