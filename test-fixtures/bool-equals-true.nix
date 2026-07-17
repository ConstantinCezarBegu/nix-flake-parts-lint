# Test for bool-equals-true rule
{ lib, ... }:

{
  config = {
    something = if (true == true) then "yes" else "no";
    something2 = if (false == false) then "yes" else "no";
    something3 = if (true != false) then "yes" else "no";
  };
}
