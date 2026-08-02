//! Integration test for rule 115: require-flake-parts
//!
//! Checks that non-flake-parts files in module paths trigger the require-flake-parts lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::RequireFlakeParts::new()));
    registry
}

#[test]
fn orphan_file_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("orphan.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 115));
}

#[test]
fn hosts_path_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("/tmp/test/hosts/myhost.nix"),
        src,
    );
    assert!(reports.is_empty());
}
