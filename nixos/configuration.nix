# example of what you can put into your configuration.nix to install hosting-farm and set it up as a service

{ config, pkgs, ... }:

let
  # Define the hosting-farm package derivation once
  hosting-farm-pkg = let
      defaultNix = builtins.fetchurl {
        # IMPORTANT: Use the raw URL of desired version of default.nix
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/refs/tags/v0.1.3/default.nix"; # UPDATE VERSION TAG IN URL IF NEEDED
        # You might need to update the hash.
        # You will get the correct hash from the error message during nixos-rebuild build
        sha256 = lib.fakeSha256; # UPDATE HASH
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
        owner = "Lab-8916100448256";
        repo = "hosting-farm";
        rev = "v0.1.3";  # UPDATE VERSION TAG IF NEEDED
        # You will get the correct hash from the error message during nixos-rebuild build
        sha256 = lib.fakeSha256; # UPDATE HASH
      };
    };
in {

  ##########################
  # your usual stuff here  #
  ##########################

  environment.systemPackages = with pkgs; [
    ############################
    # your other packages here #
    ############################

    # Add the package derivation directly
    hosting-farm-pkg
  ];

  # We define hosting-farm-pkg above. This avoids potential conflicts
  # if hosting-farm ever makes it into nixpkgs proper.

  # Persistence setup for the state directory and database files
  # Assumes you have nixos-persist or similar setup for /persist
  # If not using /persist, remove the link targets and just create the dirs/files.
  systemd.tmpfiles.rules = [
    # State directory (managed primarily by systemd's StateDirectory option below)
    # "d /var/lib/hosting-farm 0750 hosting-farm hosting-farm -" # Not needed if using StateDirectory=

    # Log directory
    # "d /var/log/hosting-farm 0750 hosting-farm hosting-farm -" # Not needed if logs are going to systemd journal

    # Asset symlink within the state directory
    # L+ creates parent directories if needed and the link itself
    "L+ /var/lib/hosting-farm/assets - - - - ${hosting-farm-pkg}/share/hosting-farm/assets"

    # Links for persistent SQLite database files (adjust if not using /persist)
    "L /var/lib/hosting-farm/hosting-farm_prod.sqlite - - - - /persist/var/lib/hosting-farm/hosting-farm_prod.sqlite"
    "L /var/lib/hosting-farm/hosting-farm_prod.sqlite-wal - - - - /persist/var/lib/hosting-farm/hosting-farm_prod.sqlite-wal"
    "L /var/lib/hosting-farm/hosting-farm_prod.sqlite-shm - - - - /persist/var/lib/hosting-farm/hosting-farm_prod.sqlite-shm"

    # Ensure parent directory for persisted files exists
    "d /persist/var/lib/hosting-farm 0750 hosting-farm hosting-farm -"
  ];

  # hosting-farm configuration file
  # As this contains sensitive information, you might want to get it from a secret instead of putting it directly in your configuration.nix
  # environment.etc."hosting-farm/production.yaml".text = "${hosting-farm-config}";
  environment.etc."hosting-farm/production.yaml" = {
    text = ''
    logger:
      enable: true
      pretty_backtrace: true
      level: info
      format: compact
    server:
      port: 5150
      binding: localhost
      host: http://localhost
      middlewares:
        static:
          enable: true
          must_exist: true
          precompressed: false
          folder:
            uri: "/static"
            path: "assets/static"
          fallback: "assets/static/404.html"
        fallback:
          enable: true
          file: "assets/static/404.html"
    workers:
      mode: BackgroundAsync
    mailer:
      smtp:
        enable: true
        host: mail.example.com
        port: 587
        secure: true
        auth:
          user: "user@example.com"
          password: "use a very safe password! (not this one)"
    database:
      uri: {{ get_env(name="DATABASE_URL", default="sqlite://hosting-farm_prod.sqlite?mode=rwc") }}
      enable_logging: false
      connect_timeout: {{ get_env(name="DB_CONNECT_TIMEOUT", default="500") }}
      idle_timeout: {{ get_env(name="DB_IDLE_TIMEOUT", default="500") }}
      min_connections: {{ get_env(name="DB_MIN_CONNECTIONS", default="1") }}
      max_connections: {{ get_env(name="DB_MAX_CONNECTIONS", default="1") }}
      auto_migrate: true
      dangerously_truncate: false
      dangerously_recreate: false
    auth:
      jwt:
        secret: P4u28pKuFldLTYSofkWu
        expiration: 604800 # 7 days
        location:
          from: Cookie
          name: auth_token
    '';
    # Set ownership and permissions
    user = "root";              # Owned by root (standard for /etc files)
    group = "hosting-farm";     # Accessible by the hosting-farm group
    mode = "0440";              # Permissions: Owner=read, Group=read, Other=no access
                                # (r--r-----)
  };


  # Dedicated user/group for the service
  users.users.hosting-farm = {
    isSystemUser = true;
    group = "hosting-farm";
    # Optional: Define home, shell etc. if needed, but usually not for system users
    # home = "/var/lib/hosting-farm"; # systemd StateDirectory handles the dir
  };
  users.groups.hosting-farm = {};

  # Systemd Service Definition
  systemd.services.hosting-farm = {
    description = "Hosting Farm web app";
    documentation = [ "https://nixin.distrilab.eu/" ]; 
    after = [ "network-online.target" ]; # Wait for network to be fully up
    wants = [ "network-online.target" ];
    wantedBy = [ "multi-user.target" ];

    environment = {
      # Point to the config file directory
      LOCO_CONFIG_FOLDER = "/etc/hosting-farm";
      # You could potentially set other env vars needed by the app here
      # e.g., RUST_LOG if not configured via the yaml
    };

    serviceConfig = {
      Type = "simple";

      # Run as dedicated user/group
      User = "hosting-farm";
      Group = "hosting-farm";

      # Let systemd manage the state directory /var/lib/hosting-farm
      # This creates the directory with correct permissions (owned by User/Group)
      StateDirectory = "hosting-farm"; # Manages /var/lib/hosting-farm
      # Optional: Specify mode if needed, defaults usually fine with User/Group set
      # StateDirectoryMode=0750

      # Logs will go here (ensure dir exists and has correct permissions)
      # LogsDirectory = "hosting-farm"; # Manages /var/log/hosting-farm
      # LogsDirectoryMode=0750

      # Set the working directory to the state directory. The app finds assets/ and writes DB here
      WorkingDirectory = "/var/lib/hosting-farm"; # Should be writable due to StateDirectory
      # But need to explicitly allow writing to the directory containing the actual DB files
      # because the target of the symlinks are outside the StateDirectory
      ReadWritePaths = [ "/persist/var/lib/hosting-farm" ];

      # Path to the executable using the package definition
      ExecStart = "${hosting-farm-pkg}/bin/hosting_farm-cli start -e production";
      Restart = "always";
      RestartSec = 10;

      # Hardening settings
      ProtectHome = true;
      PrivateTmp = true;
      PrivateDevices = true;
      ProtectHostname = true;
      ProtectClock = true;
      ProtectKernelTunables = true;
      ProtectKernelModules = true;
      ProtectKernelLogs = true;
      ProtectControlGroups = true;
      RestrictAddressFamilies = "AF_INET AF_INET6";
      RestrictNamespaces = true;
      # NoNewPrivileges=true; # Consider adding this for extra security if the app doesn't need root privileges during startup
    };
  };

  ##########################
  # your usual stuff here #
  ##########################
}

