# Test for no-cross-namespace-writes rule - should trigger lint for undeclared namespace
{ config, lib, ... }:

{
  options.myService.enable = lib.mkOption {
    type = lib.types.bool;
    description = "Enable my service";
  };

  config.myService.enable = true;

  # Writing to undeclared namespace - should trigger lint
  config.myOtherService.enabled = true;
}
