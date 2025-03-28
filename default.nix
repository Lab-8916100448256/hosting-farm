# ./default.nix
{ pkgs ? import <nixpkgs> { }, src ? ./. }:

let
  # Use the provided src, defaulting to the current directory
  theSource = src;

  # Helper to parse Cargo.toml for version and potentially pname
  # This avoids hardcoding the version in Nix if you prefer
  cargoToml = pkgs.lib.importTOML "${theSource}/Cargo.toml";

  # Get the actual binary name from Cargo.toml [[bin]] section if possible,
  # or hardcode if simpler. Assuming it's 'hosting_farm-cli'.
  binaryName = "hosting_farm-cli";
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = "hosting-farm"; # The package name
  # Fetch version from Cargo.toml
  version = cargoToml.package.version;

  # Clean the source to remove potentially unwanted files (like .git)
  src = pkgs.lib.cleanSource "${theSource}";

  # Point to the lock file within the source
  cargoLock.lockFile = "${src}/Cargo.lock"; # Use ${src} here as it's the cleaned source

  # Build and check in release mode for performance
  checkType = "release";
  buildType = "release";

  # Ensure build inputs like OpenSSL, pkg-config are available if needed
  nativeBuildInputs = with pkgs; [ pkg-config ];
  buildInputs = with pkgs; [ openssl ]; # Add other Rust dependencies if needed

  # Custom installation phase:
  # The default installPhase for rustPlatform typically just installs binaries to $out/bin.
  # We override it to also copy the assets directory.
  installPhase = ''
    runHook preInstall

    # Install binary
    mkdir -p $out/bin
    echo "Installing binary ${binaryName} to $out/bin/"
    ls $cargoArtifacts
    ls $cargoArtifacts/target
    ls $cargoArtifacts/target/realease
    install -Dm755 $cargoArtifacts/target/realease/${binaryName} $out/bin/${binaryName}

    # Install assets into standard share location
    mkdir -p $out/share/hosting-farm
    echo "Copying assets from ${src}/assets to $out/share/hosting-farm/"
    cp -r ${src}/assets $out/share/hosting-farm/

    runHook postInstall
  '';

  meta = with pkgs.lib; {
    description = "Hosting Farm web application";
    homepage = "https://github.com/Lab-8916100448256/hosting-farm";
    license = licenses.cc0; # Please verify the actual license
    maintainers = [ maintainers.lab-8916100448256 ];
  };
}

