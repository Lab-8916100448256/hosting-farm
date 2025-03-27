# Hosting Farm
A web application to manage a farm of self-hosting servers


## Developement environement
Hosting Farm is developed in Rust with the Loco-rs framework


### Debian / Ubuntu
This section describes how to install the developement dependencies of the project on a Debian GNU/Linux computer and how to build and test the application. These instructions should also work for debian based Linux distribution like Ubuntu.

1. **Installing git** 
   ```
   sudo apt update && sudo apt install git -y
   git config --global user.name "Your Name"
   git config --global user.email "you-email@example.com",
   ```

2. **Installing Rust**
   ```
   sudo apt update && sudo apt install build-essential -y
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup component add rust-analyzer rust-src
   ```


3. Installing loco-rs and sea-orm CLI tools
   ```
   sudo apt update && sudo apt install pkg-config -y
   cargo install loco
   cargo install sea-orm-cli
   cargo install sea-orm-cli
   ```

4. **Building the application**
   To build the application, run the following command :
   ```  
   cargo build
   ```

5. **Running automated tests**  
   To run unit tests, documentation tests and integration tests run the following command :  
   ```
   cargo test
   ```
   It will also build the application if it has not been already built or if the source code has changed since the last build.


6. **Starting a development instance of the application**
   To run a development instance of the application, run the following command :
   ```
   cargo loco start
   ```
   It will also build the application if it has not been already built or if the source code has changed since the last build.
   If building the application was successfull, a development instance that you can use to manually test the application with your web browser should be listening on http://localhost:5150


### NixOS
This section describes how to install the developement dependencies of the project on a NixOS.

1. **Developement shell**
   To enter a shell with all the developement dependencies installed, run `nix-shell` at the root of the project directory 
   Then you can use the same cargo commands as in the Debian section above

2. **Building the app the Nix way**
   Alternatively, you can build the application by running `nix-build` at the root of the project directory
   The build outputs will then be in ./result

3. **Install the application on a test server**
   Add something like the following to the NixOS configuration.nix of your server (read the comments to undestand what you have to customize :
   ```
   environment.systemPackages = with pkgs; [
     # ... everything else you have installed
     sample-app-rust
   ];
   nixpkgs.config.packageOverrides = pkgs: {
     sample-app-rust =
       let
         defaultNix = builtins.fetchurl {
           url = "https://github.com/Lab-8916100448256/hosting-farm/default.nix";
           sha256 = "sha256:Put the correct hash here"; # Need to customize this
         };
       in pkgs.callPackage defaultNix {
         src = pkgs.fetchFromGitHub {
           owner = "Lab-8916100448256";
           repo = "hosting-farm";
           rev = "main";  # REPLACE WITH A RELEASE TAG!
           sha256 = "sha256-Put the correct hash here"; # Need to customize this
         };
       };
   };
   ```
