//! Integration test for rule 107: no-nix-env
//!
//! Checks that `nix-env` usage triggers the no-nix-env lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoNixEnv::new()));
    registry
}

#[test]
fn nix_env_in_string_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, pkgs, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config = {
    scripts.hello.text = ''
      nix-env -i hello
    '';
  };
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 107));
}

#[test]
fn no_nix_env_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, pkgs, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.foo = pkgs.hello;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
