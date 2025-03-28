# Sample App Rust: NixOS Testbed

This contains an example of things to add to a NixOS configuration.nix to install hosting-farm and set it up as a service.
Not the there is currently a bug in the loco-rs framework that is caussing some static asset to be missing in the packaged file. 
Until this is fixed, to workaround this, you have to copy the assets folder to /opt/hosting-farm manually after installing hosting-farm.


## Testing the build from GitHub sources without needing to actually change your NixOS system configuration
This folder also allows you to test whether building the app from GitHub and running works without needing to actually change your NixOS system configuration. 

Run this command in this folder to build from GitHub:

```bash
nix-build
```

You will probably have an error about an incorrect hash. Update the default.nix file with the correct hash given by the error message and try again
As there are 2 hashes in the file you will probably have to do that 2 times

Then, when the build is successful add a configuration file to /etc/hosting-farm/production.yaml and you can run the built program with:
Run it from the project root folder if the missing assets issue has not yet been fixed

```bash
LOCO_CONFIG_FOLDER="/etc/hosting-farm ./result/bin/hosting-farm start  -e production
```
