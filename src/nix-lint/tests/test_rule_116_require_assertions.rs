//! Integration test for rule 116: require-assertions
//!
//! Checks that modules without assertions trigger the require-assertions lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::RequireAssertions::new()));
    registry
}

#[test]
fn options_no_assertions_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.myService.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
  config.myService.bar = true;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 116));
}

#[test]
fn options_with_assertions_block_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  assertions = [
    { condition = config.foo; }
  ];
  config.foo = true;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(reports.is_empty());
}

#[test]
fn options_with_assert_stmt_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
  assert config.foo;
  {
    options.foo = lib.mkOption { type = lib.types.bool; };
    config.foo = true;
  }"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(reports.is_empty());
}

#[test]
fn no_options_no_trigger() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  config.foo = "bar";
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(reports.is_empty());
}
