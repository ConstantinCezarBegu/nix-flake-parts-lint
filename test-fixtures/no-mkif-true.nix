# Test for no-mkif-true rule
{ lib, ... }:

{
  config = {
    something = lib.mkIf true "value";
  };
}
