{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let
  # Use the provided src, defaulting to the current directory
  theSource = src;

  # Helper to parse Cargo.toml for version and potentially pname
  # This avoids hardcoding the version in Nix if you prefer
  cargoToml = pkgs.lib.importTOML "${theSource}/Cargo.toml";

in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "hosting-farm";
  # Fetch version from Cargo.toml
  version = cargoToml.package.version;

  # Clean the source to remove potentially unwanted files (like .git)
  src = pkgs.lib.cleanSource "${theSource}";

  # Point to the lock file within the source
  cargoLock.lockFile = "${src}/Cargo.lock"; # Use ${src} here as it's the cleaned source

  # Build and check in release mode for performance
  checkType = "release";
  buildType = "release";

  # Custom installation phase:
  # The default installPhase for rustPlatform typically just installs binaries to $out/bin.
  # We override it to also copy the assets directory.
  installPhase = ''
    runHook preInstall

    # Create the target directory structure within the Nix store output ($out)
    # We'll mirror the desired /opt/hosting-farm structure here
    mkdir -p $out/opt/hosting-farm
    mkdir -p $out/bin

    # Copy the assets directory from the source into the package output
    # Use cp -r for recursive copy
    echo "Copying assets from ${src}/assets to $out/opt/hosting-farm/"
    cp -r ${src}/assets $out/opt/hosting-farm/

    # Install the main binary (assuming the binary name matches pname)
    # $cargoArtifacts points to target/release (or target/debug) directory
    echo "Installing binary ${pname} to $out/bin/"
    install -Dm755 $cargoArtifacts/bin/${pname} $out/bin/${pname}

    runHook postInstall
  '';

  # Optional: Add metadata about the package
  meta = with pkgs.lib; {
    description = "Hosting Farm web application";
    homepage = "https://github.com/Lab-8916100448256/hosting-farm";
    # Replace with your actual license if different
    license = licenses.mit; # Or licenses.unfree if source isn't available/licensed freely
    # Add maintainers if desired
    # maintainers = [ maintainers.yourGithubUsername ];
    # Specify platforms if needed, though Rust often works on many
    platforms = platforms.linux; # Example
  };
}

