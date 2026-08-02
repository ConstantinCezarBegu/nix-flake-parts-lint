//! Integration test for rule 104: no-mkif-true
//!
//! Checks that `mkIf true` usage triggers the no-mkif-true lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoMkIfTrue::new()));
    registry
}

#[test]
fn mkIf_true_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = lib.mkIf true { bar = 42; };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 104));
}

#[test]
fn mkIf_variable_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, config, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = lib.mkIf config.bar { baz = 42; };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
