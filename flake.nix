{
  description = "nix-flake-parts-lint: a static analyzer for Nix flake-parts configurations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs@{ flake-parts, nixpkgs, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [];

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', pkgs, system, ... }: {
        checks = {
          lint = self'.packages.nix-flake-parts-lint;
          default = self'.checks.lint;
        };

        packages = {
          nix-flake-parts-lint = pkgs.rustPlatform.buildRustPackage {
            pname = "nix-flake-parts-lint";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
            ];

            buildInputs = with pkgs; [
              gcc
            ];

            meta = with pkgs.lib; {
              description = "A static analyzer for Nix flake-parts configurations";
              homepage = "https://github.com/ConstantinCezarBegu/nix-flake-parts-lint";
              license = licenses.mit;
              platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
              mainProgram = "nix-flake-parts-lint";
            };
          };
          default = self'.packages.nix-flake-parts-lint;
        };

        apps = {
          lint = {
            type = "app";
            program = "${self'.packages.nix-flake-parts-lint}/bin/nix-flake-parts-lint";
          };
          default = self'.apps.lint;
        };

        devShells.default = pkgs.mkShell {
          name = "nix-flake-parts-lint-dev";

          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            cargo-watch
          ];
        };
      };
    };
}
