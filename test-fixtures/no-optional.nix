# Test for no-optional rule
{ lib, ... }:

{
  config = {
    something = lib.optional true "value";
    something2 = lib.optionally true "value";
    something3 = lib.optionalString true "value";
  };
}
