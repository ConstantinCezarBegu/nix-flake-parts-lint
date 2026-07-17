# Test for no-mkforce rule
{ lib, ... }:

{
  options = {
    myOption = lib.mkOption {
      default = lib.mkForce "forced";
    };
  };
}
