{ pkgs ? import <nixpkgs> { } }:

let
  hosting-farm =
    let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/refs/heads/dev-cursor/default.nix";
        sha256 = "sha256:0zqsh1ayfb0818j8kv1yy1dfyhgbwypgaq6yz5d1nc418f8kk5zs"; # Need to customize this
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
           owner = "Lab-8916100448256";
           repo = "hosting-farm";
           rev = "2a8dc048fcd0161da67720903c286fa85dae8342";  # REPLACE WITH A RELEASE TAG!
           sha256 = "sha256-024c96sg6b7w56jpixl388kz9qsxll9avpymmg6v8lkbsg4na1lx"; # Need to customize this
      };
    };
  in [
    hosting-farm
  ]
