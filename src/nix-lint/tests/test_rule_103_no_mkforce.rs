//! Integration test for rule 103: no-mkforce
//!
//! Checks that `mkForce` usage triggers the no-mkforce lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoMkForce::new()));
    registry
}

#[test]
fn mkForce_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, config, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = lib.mkForce true;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 103));
}

#[test]
fn normal_assignment_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = true;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
