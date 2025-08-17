{
  inputs.nixpkgs.url = "github:nixos/nixpkgs";
  inputs.pinnacle.url = "github:cassandracomar/pinnacle";
  inputs.crate2nix.url = "github:Pacman99/crate2nix/fix-subcrates";
  inputs.crate2nix.inputs.nixpkgs.follows = "nixpkgs";
  inputs.crate2nix.inputs.crate2nix_stable.follows = "crate2nix";

  outputs = {nixpkgs, pinnacle, crate2nix, ...}: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
    };
  in {
    formatter.x86_64-linux = pkgs.alejandra;

    packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
      pname = "pinnacle-config";
      version = "0.1.0";
      src = ./.;

      PINNACLE_PROTOBUF_API_DEFS = "${pinnacle}/api/protobuf";
      PINNACLE_PROTOBUF_SNOWCAP_API_DEFS = "${pinnacle}/snowcap/api/protobuf";

      nativeBuildInputs = with pkgs; [protobuf pkg-config];
      buildInputs = with pkgs; [
        seatd.dev
        libxkbcommon
        libinput
        lua5_4
        libdisplay-info
        libgbm
      ];

      cargoLock = {
        lockFile = ./Cargo.lock;
        allowBuiltinFetchGit = true;
      };
    };

    devShells.${system}.default = pkgs.mkShell {
      packages = with pkgs; [rustc cargo rust-analyzer clang protobuf libxkbcommon pkg-config];
    };
  };
}
