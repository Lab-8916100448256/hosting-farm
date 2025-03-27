{ pkgs ? import <nixpkgs> { } }:
let 
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
}


# version with hard-coded package name and version
# pkgs.rustPlatform.buildRustPackage rec {
#   pname = "hosting-farm";
#   version = "0.1.0";
#   cargoLock.lockFile = ./Cargo.lock;
#   src = pkgs.lib.cleanSource ./.;
# }
