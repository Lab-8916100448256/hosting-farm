# Hosting Farm
POC for a web application to manage a farm of self-hosting servers


## Developement environement
Hosting Farm is developed in Rust with the Loco-rs framework


### Radicle
To clone this repository on Radicle, simply run:
```
rad clone rad:z4U7rokqyLuMj1dPSQsakczPTwsPB
```

### Debian / Ubuntu
This section describes how to install the developement dependencies of the project on a Debian GNU/Linux computer and how to build and test the application.  
These instructions should also work for debian based Linux distribution like Ubuntu.

1. **Installing git and other project requirements** 
   ```
   sudo apt update && sudo apt install git nettle-dev libclang-dev -y
   git config --global user.name "Your Name"
   git config --global user.email "you-email@example.com",
   ```

2. **Installing Rust**
   ```
   sudo apt update && sudo apt install build-essential -y
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup component add rust-analyzer rust-src
   ```

3. Installing loco-rs and other CLI tools
   ```
   sudo apt update && sudo apt install pkg-config -y
   cargo install loco
   cargo install sea-orm-cli
   cargo install cargo-insta
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
     hosting-farm
   ];
   nixpkgs.config.packageOverrides = pkgs: {
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
           rev = "dev-cursor";  # REPLACE WITH A RELEASE TAG!
           sha256 = "sha256-0zqsh1ayfb0818j8kv1yy1dfyhgbwypgaq6yz5d1nc418f8kk5zs"; # Need to customize this
         };
       };
   };
   ```


### End-to-end tests
#### To execute the end-to-end tests
Run the tests with : 
```
npx playwright test
```
Before executing thet tests, Playwright will automatically start the application with the e2e profile which listen on the port 5151 and is using a test DB to avoid conflicts with your development instance.
The application will be killed at the end of the tests.

To start playwright tests in interactive UI mode.
```
npx playwright test --ui
```

#### Auto generate test code with Codegen 
First, in one terminal, start the application with the e2e profile:
```
cargo loco start -e e2e
```
Then, in a second terminal, start playwright in codegen mode
```
npx playwright codegen http://localhost:5151
```

For more information on writing tests, check Playwright documentation : https://playwright.dev/docs/intro

#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
#
