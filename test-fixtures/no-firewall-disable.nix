# Test for no-firewall-disable rule
{ ... }:

{
  networking.firewall.enable = false;
}
