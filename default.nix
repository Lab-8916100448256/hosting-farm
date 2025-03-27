{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let 
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
  theSource = src;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;
  src = pkgs.lib.cleanSource ./.;
  cargoLock.lockFile = ./Cargo.lock;
  #src = pkgs.lib.cleanSource "${theSource}";
  #cargoLock.lockFile = "${theSource}/Cargo.lock";
}


# version with hard-coded package name and version
# pkgs.rustPlatform.buildRustPackage rec {
#   pname = "hosting-farm";
#   version = "0.1.0";
#   cargoLock.lockFile = ./Cargo.lock;
#   src = pkgs.lib.cleanSource ./.;
# }
