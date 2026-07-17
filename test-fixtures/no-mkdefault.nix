# Test for no-mkdefault rule
{ lib, ... }:

{
  options = {
    myOption = lib.mkOption {
      default = "value";
    };
  };
}
