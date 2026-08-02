//! Integration test for rule 118: no-cross-module-option-reads
//!
//! Checks that config reads from undeclared namespaces trigger the no-cross-module-option-reads lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossModuleOptionReads::new()));
    registry
}

#[test]
fn cross_module_read_triggers() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }:
{
  options.myService.foo = lib.mkOption {
    type = lib.types.bool;
    description = "A test option";
  };
  config.myService.bar = config.otherService.baz;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake/modules/test.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 118));
}

#[test]
fn cross_module_assert_triggers() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  assert config.otherService.enabled;
  {
    config.myService.bar = true;
  }
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("test.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 118));
}

#[test]
fn same_namespace_no_trigger() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.bar = config.myService.foo;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("test.nix"),
        src,
    );
    assert!(reports.is_empty());
}

#[test]
fn builtin_nix_read_no_trigger() {
    let registry = make_registry();
    let src = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.nixSetting = config.nix.settings.auto-optimise-store;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("test.nix"),
        src,
    );
    assert!(reports.is_empty());
}
