//! Integration test for rule 106: no-lookup-path
//!
//! Checks that `import <nixpkgs>` usage triggers the no-lookup-path lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoLookupPath::new()));
    registry
}

#[test]
fn import_nixpkgs_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
  config = {
    bar = import <nixpkgs>;
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 106));
}

#[test]
fn import_file_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
  config.foo = import ./foo.nix;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
