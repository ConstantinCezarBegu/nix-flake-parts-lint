# Clean flake-parts module - should not trigger any cross-namespace issues
{ config, lib, self', inputs', pkgs, ... }:

{
  options.myService.enable = lib.mkOption {
    type = lib.types.bool;
    description = "Enable my service";
  };

  config.myService.enable = true;
  config.nix.settings.auto-optimise-store = true;
  config.services.nginx.enable = true;
  config.programs.git.enable = true;

  assertions = [
    {
      message = "must be enabled";
      check = config.myService.enable;
    }
  ];
}
