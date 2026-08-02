# Test for no-cross-namespace-writes rule with built-in options
# Uses dotted path style for config writes (matching the regex pattern)
{ config, lib, ... }:

{
  options.myService.enable = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Enable my service";
  };

  options.myService.package = lib.mkOption {
    type = lib.types.package;
    description = "The package for my service";
  };

  config.myService.enable = true;
  config.myService.package = pkgs.myapp;

  # Built-in nixpkgs options - should not trigger lint
  config.nix.settings.auto-optimise-store = true;
  config.services.nginx.enable = true;
  config.programs.git.enable = true;
  config.darwin.enableAutologin = true;
}
