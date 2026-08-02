//! Integration tests that exercise the full lint pipeline using the library API.
//!
//! These tests cover the same scenarios as the original shell-based tests but
//! use the library directly (no binary build required).

use nix_lint_core::{LintRegistry, Severity};

fn make_full_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();

    registry.register(Box::new(nix_lint_rules::NoRec::new()));
    registry.register(Box::new(nix_lint_rules::NoWithPkgsLib::new()));
    registry.register(Box::new(nix_lint_rules::NoMkDefault::new()));
    registry.register(Box::new(nix_lint_rules::NoMkForce::new()));
    registry.register(Box::new(nix_lint_rules::NoMkIfTrue::new()));
    registry.register(Box::new(nix_lint_rules::NoAnyType::new()));
    registry.register(Box::new(nix_lint_rules::NoLookupPath::new()));
    registry.register(Box::new(nix_lint_rules::NoNixEnv::new()));
    registry.register(Box::new(nix_lint_rules::NoOptional::new()));
    registry.register(Box::new(nix_lint_rules::NoFirewallDisable::new()));
    registry.register(Box::new(nix_lint_rules::NoBuiltinReadfileSecrets::new()));
    registry.register(Box::new(nix_lint_rules::NoMissingDescription::new()));
    registry.register(Box::new(nix_lint_rules::NoSecrets::new()));
    registry.register(Box::new(nix_lint_rules::NoDefaults::new()));
    registry.register(Box::new(nix_lint_rules::BoolEqualsTrue::new()));
    registry.register_file_level(Box::new(nix_lint_rules::OneProgramPerPart::new()));
    registry.register_file_level(Box::new(nix_lint_rules::RequireFlakeParts::new()));
    registry.register_file_level(Box::new(nix_lint_rules::RequireAssertions::new()));
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossNamespaceWrites::new()));
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossModuleOptionReads::new()));

    registry
}

#[test]
fn lint_rec_attrset_returns_violation() {
    let registry = make_full_registry();
    let src = r#"rec {
  foo = bar;
  bar = 42;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 100));
}

#[test]
fn lint_clean_file_returns_no_violations() {
    let registry = make_full_registry();
    let src = r#"{ lib, ... }: {
  options.myOption = lib.mkOption {
    description = "A test option";
    type = lib.types.bool;
  };

  config.myOption = false;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}

#[test]
fn lint_with_pkgs_returns_violation() {
    let registry = make_full_registry();
    let src = r#"{ lib, pkgs, ... }:
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
fn lint_list_shows_all_rules() {
    let registry = make_full_registry();
    let all_rules: Vec<_> = registry.lints().iter().map(|l| l.name()).collect();
    let file_rules: Vec<_> = registry.file_level_rules().iter().map(|r| r.name()).collect();

    assert!(!all_rules.is_empty(), "Should have AST-based lint rules");
    assert!(!file_rules.is_empty(), "Should have file-level rules");
    assert!(all_rules.contains(&"no-rec"));
    assert!(file_rules.contains(&"one-program-per-part"));
}

#[test]
fn lint_explain_provides_details() {
    let registry = make_full_registry();
    let no_rec = registry.lints().iter().find(|l| l.code() == 100).unwrap();
    let explain: &dyn nix_lint_core::Explain = no_rec.as_ref();
    let explanation = explain.explanation();
    assert!(explanation.contains("rec"));
}

#[test]
fn lint_all_severity_levels_present() {
    let registry = make_full_registry();
    let file_severities: Vec<_> = registry.file_level_rules().iter().map(|r| r.severity()).collect();
    assert!(file_severities.contains(&Severity::Warn), "File-level rules should have Warn");
    assert!(file_severities.contains(&Severity::Error), "File-level rules should have Error");
}
