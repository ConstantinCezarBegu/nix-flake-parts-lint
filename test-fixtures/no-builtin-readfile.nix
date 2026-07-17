# Test for no-builtin-readfile-secrets rule
{ ... }:

let
  secret = builtins.readFile ./passwords.txt;
in
{
  something = secret;
}
