{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let theSource = src; in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "hosting-farm";
  version = "0.1.1";
  src = pkgs.lib.cleanSource "${theSource}";
  cargoLock.lockFile = "${theSource}/Cargo.lock";
}
