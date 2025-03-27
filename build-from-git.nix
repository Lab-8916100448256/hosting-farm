{ pkgs ? import <nixpkgs> { } }:

let
  hosting-farm =
    let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/a748152750c5322c0865f5099ea9f79d0b5a4ca2/default.nix";
        #sha256 = "sha256:0zqsh1ayfb0818j8kv1yy1dfyhgbwypgaq6yz5d1nc418f8kk5zs"; # Need to customize this
        sha256 = "4d6cc78883d224379ea7361f62c445c60c12b4f75e8d02c6da2af8996f914ef1"; # Need to customize this
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
           owner = "Lab-8916100448256";
           repo = "hosting-farm";
           rev = "a748152750c5322c0865f5099ea9f79d0b5a4ca2";  # REPLACE WITH A RELEASE TAG!
           sha256 = "sha256-024c96sg6b7w56jpixl388kz9qsxll9avpymmg6v8lkbsg4na1lx"; # Need to customize this
      };
    };
  in [
    hosting-farm
  ]
