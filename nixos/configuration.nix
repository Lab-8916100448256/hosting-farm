# example of what you can put into your configuration.nix to install hosting-farm and set it up as a service

{ config, pkgs, ... }:

in {

  ##########################
  # your usual stuffe here #
  ##########################

  environment.systemPackages = with pkgs; [

    ############################
    # your usual packages here #
    ############################

    hosting-farm
  ];

  nixpkgs.config.packageOverrides = pkgs: {
    hosting-farm = let
      defaultNix = builtins.fetchurl {
        url = "https://raw.githubusercontent.com/Lab-8916100448256/hosting-farm/refs/heads/dev-cursor/default.nix";
        sha256 = "sha256:1a3dgnkz5kvc9axj2v7ybh7w08qap21fq3n856fldwh9351q969y"; #### You will have to update the hash here. Run nixos-rebuild and pick the hash from the error
      };
    in pkgs.callPackage defaultNix {
      src = pkgs.fetchFromGitHub {
        owner = "Lab-8916100448256";
        repo = "hosting-farm";
        rev = "dev-cursor";  # REPLACE WITH A RELEASE TAG for production deployment!
        sha256 = "sha256-0UqD4J18rbysxkursEzm/iTmIYVc0/rzOCktNOohhFA="; #### You will have to update the hash here. Run nixos-rebuild and pick the hash from the error
      } + "/.";
    };
  };

  systemd.tmpfiles.rules = [
    "d /opt/hosting-farm 0700 root root"
    "d /var/log/hosting-farm 0700 root root"
    "L /opt/hosting-farm/hosting-farm_prod.sqlite - - - - /persist/opt/hosting-farm/hosting-farm_prod.sqlite"
    "L /opt/hosting-farm/hosting-farm_prod.sqlite-wal - - - - /persist/opt/hosting-farm/hosting-farm_prod.sqlite-wal"
    "L /opt/hosting-farm/hosting-farm_prod.sqlite-shm - - - - /persist/opt/hosting-farm/hosting-farm_prod.sqlite-shm"
  ];

  #environment.etc."hosting-farm/production.yaml".text = "${hosting-farm-config}";
  environment.etc."hosting-farm/production.yaml".text = ''
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

  systemd.services.hosting-farm = {
    description = "Hosting Farm web app";
    documentation = [ 
      "https://nixin.distrilab.eu/"
    ];
    after = [ "network-pre.target" ];
    wants = [ "network-pre.target" ];
    wantedBy = [ "multi-user.target" ];
    environment = {
      LOCO_CONFIG_FOLDER = "/etc/hosting-farm";
    };
    serviceConfig = {
      Type = "simple";
      #LimitNOFILE=1000000;
      WorkingDirectory="/opt/hosting-farm";
      ExecStart="${pkgs.hosting-farm}/bin/hosting_farm-cli start -e production";
      Restart="always";
      RestartSec=10;
    };
  };

  ##########################
  # your usual stuffe here #
  ##########################

}
