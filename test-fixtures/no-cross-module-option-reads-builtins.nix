# Test for no-cross-module-option-reads rule with built-in options
# Uses dotted path style for config reads (matching the regex pattern)
{ config, lib, ... }:

{
  options.myService.foo = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "My service foo";
  };

  options.myService.bar = lib.mkOption {
    type = lib.types.str;
    description = "My service bar";
  };

  config.myService.bar = if config.nix.settings.auto-optimise-store then "optimised" else "not";
  config.myService.foo = config.services.nginx.enable;
  assert config.flake.description != "";
  config.myService.bar = "ok";
}
