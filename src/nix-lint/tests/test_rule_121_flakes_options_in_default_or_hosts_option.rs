//! Integration test for rule 121: flakes-options-in-default-or-hosts-option
//!
//! Every options.X namespace must be in its own X-option.nix file.
//! default.nix is allowed for shared configs.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::FlakesOptionsInDefaultOrHostsOption::new()));
    registry
}

#[test]
fn default_nix_valid() {
    let registry = make_registry();
    let src = r#"{ lib }: {
  options.git.autocrlf = lib.mkOption { type = lib.types.str; };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("development/git/default.nix"),
        src,
    );
    assert!(reports.is_empty(), "default.nix should be valid");
}

#[test]
fn hosts_option_nix_valid() {
    let registry = make_registry();
    let src = r#"{ lib }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("hosts/myService-option.nix"),
        src,
    );
    assert!(reports.is_empty(), "hosts/*-option.nix should be valid");
}

#[test]
fn hosts_config_nix_invalid() {
    let registry = make_registry();
    let src = r#"{ lib }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("hosts/config.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 121));
    assert!(reports.iter().any(|r| r.message.contains("myService-option.nix")));
}

#[test]
fn git_nix_invalid() {
    let registry = make_registry();
    let src = r#"{ lib }: {
  options.git.autocrlf = lib.mkOption { type = lib.types.str; };
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("development/git/git.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 121));
    assert!(reports.iter().any(|r| r.message.contains("git-option.nix")));
}

#[test]
fn no_options_no_report() {
    let registry = make_registry();
    let src = r#"{ pkgs }: {
  packages.myPackage = pkgs.hello;
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("default.nix"),
        src,
    );
    assert!(reports.is_empty());
}
