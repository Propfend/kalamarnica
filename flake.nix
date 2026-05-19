{
  description = "Simple and opinionated CLI tool which changes github contexts - Accounts and per-account tokens (permissions). It currently supports Github and Gitlab.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      packages.default = pkgs.rustPlatform.buildRustPackage {
        pname = "kalamarnica";
        version = "0.2.0";

        src = ./.;

        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = [pkgs.pkg-config];

        buildInputs = [
          pkgs.openssl
        ];

        meta = {
          description = "Simple and opinionated CLI tool which changes github contexts - Accounts and per-account tokens (permissions). It currently supports Github and Gitlab.";
          homepage = "https://github.com/fangen/kalamarnica";
          license = pkgs.lib.licenses.asl20;
        };
      };
    });
}
