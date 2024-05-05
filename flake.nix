{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    {
      overlays.default =
        final: _:
        let
          cargo = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        in
        {
          audio-lint = final.rustPlatform.buildRustPackage {
            pname = "${cargo.package.name}";
            version = "${cargo.package.version}";
            src = ./.;
            cargoHash = "sha256-VYJAENFIcSO78RwpZr4bI+87RrdgshIbxYTIlcZWy5A=";
          };
        };
    }
    // flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ self.overlays.default ];
        };
        inherit (pkgs) lib;
      in
      {
        packages.default = pkgs.audio-lint;

        formatter = pkgs.nixfmt;

        devShell = pkgs.mkShell {
          buildInputs = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.rust-analyzer
            pkgs.pre-commit
            pkgs.rustPackages.clippy
          ];

          nativeBuildInputs = lib.optionals pkgs.stdenv.isDarwin ([
            pkgs.darwin.libiconv
            pkgs.darwin.apple_sdk.frameworks.Foundation
          ]);

          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        };
      }
    );
}
