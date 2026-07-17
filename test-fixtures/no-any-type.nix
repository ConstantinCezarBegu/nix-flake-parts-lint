# Test for no-any-type rule
{ lib, ... }:

{
  options = {
    myOption = lib.mkOption {
      type = lib.types.anything;
    };
  };
}
