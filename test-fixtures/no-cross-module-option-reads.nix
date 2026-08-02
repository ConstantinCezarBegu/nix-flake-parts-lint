# Test for no-cross-module-option-reads rule - should trigger lint for undeclared namespace
{ config, lib, ... }:

{
  options.myService.foo = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "My service foo";
  };

  config.myService.bar = config.otherService.baz;
}
