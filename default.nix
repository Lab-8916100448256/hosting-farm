# ./default.nix
{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let
  # Use the provided src, defaulting to the current directory
  theSource = src;

  # Helper to parse Cargo.toml for version and potentially pname
  # This avoids hardcoding the version in Nix if you prefer
  cargoToml = pkgs.lib.importTOML "${theSource}/Cargo.toml";

  # The default install hook usually finds the binary/binaries defined in Cargo.toml
  # It will install them to $out/bin. We don't need to specify the name here
  # unless there are multiple binaries and we want specific handling.
  # binaryName = "hosting_farm-cli";
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "hosting-farm"; # The package name
  # Fetch version from Cargo.toml
  version = cargoToml.package.version;

  # Clean the source to remove potentially unwanted files (like .git)
  src = pkgs.lib.cleanSource "${theSource}";
  cargoLock.lockFile = "${src}/Cargo.lock";

  # Build and check in release mode for performance
  checkType = "release";
  buildType = "release";

  # Ensure build inputs like OpenSSL, pkg-config are available if needed
  # Add other native build dependencies your Rust code might need here
  nativeBuildInputs = with pkgs; [ pkg-config ];
  # Add other runtime C libraries your Rust code links against here
  buildInputs = with pkgs; [ openssl ]; # Example: if using rustls with native-certs or openssl crate

  # Use postInstall hook to add steps *after* the default installation
  postInstall = ''
    # The default installPhase (using cargoInstallHook) has already run
    # and placed the binary (e.g., hosting_farm-cli) into $out/bin.

    # Now, just copy the assets directory.
    # ${src} refers to the cleaned source directory.
    mkdir -p $out/share/hosting-farm
    echo "Copying assets from ${src}/assets to $out/share/hosting-farm/"
    cp -r ${src}/assets $out/share/hosting-farm/

    # Optional: Add any other post-installation steps here if needed.
    # For example, installing documentation, example configs, etc.
    # mkdir -p $out/share/doc/${pname}
    # cp ${src}/README.md $out/share/doc/${pname}/
  '';

  meta = with pkgs.lib; {
    description = "Hosting Farm web application";
    homepage = "https://github.com/Lab-8916100448256/hosting-farm";
    license = licenses.mit; # Please verify the actual license
    platforms = platforms.all; # Rust is generally portable
    maintainers = [ maintainers.lab-8916100448256 ]; 
  };
}

