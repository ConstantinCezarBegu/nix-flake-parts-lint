//! Integration test for rule 112: no-secrets
//!
//! Checks that secrets in config trigger the no-secrets lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoSecrets::new()));
    registry
}

#[test]
fn secrets_in_config_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config = {
    apiKey = "AKIAIOSFODNN7EXAMPLE";
    privateKey = "-----BEGIN RSA PRIVATE KEY-----";
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 112));
}

#[test]
fn no_secrets_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = "normal-value";
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
