//! Integration test for rule 117: no-cross-namespace-writes
//!
//! Checks that config writes to undeclared namespaces trigger the no-cross-namespace-writes lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossNamespaceWrites::new()));
    registry
}

#[test]
fn cross_namespace_write_triggers() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }:
{
  options.myService.enable = lib.mkOption {
    type = lib.types.bool;
    description = "Enable my service";
  };
  config.myOtherService.enabled = true;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 117));
}

#[test]
fn same_namespace_no_trigger() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.foo = true;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("test.nix"),
        src,
    );
    assert!(reports.is_empty());
}

#[test]
fn builtin_service_no_trigger() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.services.nginx.enable = true;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("test.nix"),
        src,
    );
    assert!(reports.is_empty());
}
