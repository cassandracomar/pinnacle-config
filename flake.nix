{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";
  inputs.pinnacle.url = "github:cassandracomar/pinnacle";

  outputs = {nixpkgs, pinnacle, ...}: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [
        pinnacle.overlays.${system}.default
        (final: prev: {
          pinnacle-config = prev.pinnacle.buildRustConfig {
            pname = "pinnacle-config";
            version = "0.1.0";
            src = ./.;
          };
        })
      ];
    };
  in {
    formatter.x86_64-linux = pkgs.alejandra;

    packages.x86_64-linux = {
      inherit (pkgs) pinnacle-config;
      default = pkgs.pinnacle-config;
    };

    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [rustc cargo rust-analyzer clang protobuf libxkbcommon pkg-config clippy];
    };
  };
}
