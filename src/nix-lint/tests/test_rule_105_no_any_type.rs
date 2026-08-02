//! Integration test for rule 105: no-any-type
//!
//! Checks that `lib.types.anything` usage triggers the no-any-type lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoAnyType::new()));
    registry
}

#[test]
fn anything_type_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.anything;
    description = "A test option";
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 105));
}

#[test]
fn bool_type_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
