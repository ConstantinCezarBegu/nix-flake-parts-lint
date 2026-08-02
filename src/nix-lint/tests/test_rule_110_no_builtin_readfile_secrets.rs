//! Integration test for rule 110: no-builtin-readfile-secrets
//!
//! Checks that `builtins.readFile` usage triggers the no-builtin-readfile-secrets lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoBuiltinReadfileSecrets::new()));
    registry
}

#[test]
fn builtins_readFile_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config = {
    key = builtins.readFile ./secret.key;
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 110));
}

#[test]
fn no_builtins_readFile_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = "hello";
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
