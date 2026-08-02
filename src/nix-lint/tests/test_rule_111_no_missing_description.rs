//! Integration test for rule 111: no-missing-description
//!
//! Checks that mkOption without description triggers the no-missing-description lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoMissingDescription::new()));
    registry
}

#[test]
fn missing_description_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    default = false;
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 111));
}

#[test]
fn with_description_no_trigger() {
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
