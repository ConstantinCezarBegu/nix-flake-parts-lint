# Test for no-with-pkgs-lib rule
{ lib, pkgs, ... }:

let
  hello = with pkgs; hello;
in
{
  options = {
    test = lib.mkOption { type = lib.types.bool; };
  };
  
  config = {
    test = true;
  };
}
