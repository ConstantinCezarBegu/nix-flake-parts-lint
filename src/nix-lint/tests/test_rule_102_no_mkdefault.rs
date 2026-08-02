//! Integration test for rule 102: no-mkdefault
//!
//! Checks that `mkDefault` usage triggers the no-mkdefault lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoMkDefault::new()));
    registry
}

#[test]
fn mkDefault_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = lib.mkDefault true;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 102));
}

#[test]
fn mkOverride_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = lib.mkOverride 500 true;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}

#[test]
fn lib_mkDefault_triggers() {
    let registry = make_registry();
    let src = r#"lib.mkDefault "value""#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert_eq!(reports[0].code, 102);
}
