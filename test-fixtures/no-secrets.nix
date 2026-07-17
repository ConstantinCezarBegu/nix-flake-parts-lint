# Test for no-secrets rule
{ ... }:

{
  config = {
    mysecret = "sk-1234567890abcdef";
    api_key = "AKIAIOSFODNN7EXAMPLE";
    privateKey = "-----BEGIN RSA PRIVATE KEY-----";
  };
}
