# Test for no-nix-env rule
{ pkgs, ... }:

{
  environment.systemPackages = [
    pkgs.hello
  ];
  
  # This should trigger
  system.activationScripts = ''
    nix-env -iA nixpkgs.hello
  '';
}
