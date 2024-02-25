{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.11";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    utils,
    naersk,
  }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
      inherit (pkgs) lib;
      naersk-lib = pkgs.callPackage naersk {};
    in {
      packages.default = naersk-lib.buildPackage ./.;

      formatter = pkgs.alejandra;

      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          rustfmt
          rust-analyzer
          pre-commit
          rustPackages.clippy
        ];

        nativeBuildInputs =
          lib.optionals pkgs.stdenv.isDarwin
          (with pkgs.darwin; [
            libiconv
            apple_sdk.frameworks.Foundation
          ]);

        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
      };
    });
}
