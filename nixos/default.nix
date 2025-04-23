{ pkgs ? import <nixpkgs> { } }:

let
  hosting-farm =
    let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/refs/tags/v0.1.12/default.nix"; #### update release tag here ####
        sha256 = lib.fakeSha256;
        #sha256 = "1yzhv7zfc4rimjcd37i9i7agrc93c1yi4nl8jybal8xs3pfsxhkq";  #### You will have to replace the fakeSha256 and update the hash here. Run nix-build and pick the hash from the error
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
        owner = "Lab-8916100448256";
        repo = "hosting-farm";
        rev = "v0.1.12"; #### update release tag here ####
        sha256 = lib.fakeSha256;
        #sha256 = "sha256-0UqD4J18rbysxkursEzm/iTmIYVc0/rzOCktNOohhFA="; #### You will have to replace the fakeSha256 and update the hash here. Run nix-build and pick the hash from the error
      } + "/.";
    };
  in [
    hosting-farm
  ]
