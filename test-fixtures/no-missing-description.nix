# Test for no-missing-description rule
{ lib, ... }:

{
  options = {
    myOption = lib.mkOption {
      type = lib.types.str;
    };
  };
}
