//! Integration test for rule 101: no-with-pkgs-lib
//!
//! Checks that `with pkgs` usage triggers the no-with-pkgs-lib lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoWithPkgsLib::new()));
    registry
}

#[test]
fn with_pkgs_triggers() {
    let registry = make_registry();
    let src = r#"{ pkgs, lib, ... }:
let
  hello = with pkgs; hello;
in {
  options.test = lib.mkOption {
    description = "A test option";
    type = lib.types.bool;
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 101));
}

#[test]
fn no_with_pkgs_no_trigger() {
    let registry = make_registry();
    let src = r#"{ pkgs, lib, ... }:
{
  options.test = lib.mkOption {
    description = "A test option";
    type = lib.types.bool;
  };
  config.test = pkgs.hello;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
