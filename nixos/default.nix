{ pkgs ? import <nixpkgs> { } }:

let
  hosting-farm =
    let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/refs/heads/dev-cursor/default.nix";
        sha256 = "sha256:1a3dgnkz5kvc9axj2v7ybh7w08qap21fq3n856fldwh9351q969y"; #### You will have to update the hash here. Run nix-build and pick the hash from the error
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
        owner = "Lab-8916100448256";
        repo = "hosting-farm";
        rev = "dev-cursor";  # REPLACE WITH A RELEASE TAG!
        sha256 = "sha256-0UqD4J18rbysxkursEzm/iTmIYVc0/rzOCktNOohhFA="; #### You will have to update the hash here. Run nix-build and pick the hash from the error
      } + "/.";
    };
  in [
    hosting-farm
  ]
