//! Integration test for rule 113: no-defaults
//!
//! Checks that mkOption with `default = ` triggers the no-defaults lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoDefaults::new()));
    registry
}

#[test]
fn mkoption_with_default_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "A test option";
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 113));
}

#[test]
fn sources_default_no_trigger() {
    let registry = make_registry();
    let src = r#"{
      sources.default = [ pkgs.nixvim ];
    }"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}

#[test]
fn attrset_no_default_no_trigger() {
    let registry = make_registry();
    let src = r#"{
      foo = "bar";
    }"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
