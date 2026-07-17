# Test for no-defaults rule
{ lib, ... }:

{
  options = {
    myOption = lib.mkOption {
      default = "value";
    };
  };
}
