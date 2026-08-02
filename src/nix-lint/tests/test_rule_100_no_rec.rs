//! Integration test for rule 100: no-rec
//!
//! Checks that `rec {}` usage triggers the no-rec lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoRec::new()));
    registry
}

#[test]
fn rec_attrset_triggers() {
    let registry = make_registry();
    let src = r#"rec {
      foo = bar;
      bar = 42;
    }"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].code, 100);
}

#[test]
fn normal_attrset_no_trigger() {
    let registry = make_registry();
    let src = r#"{
      foo = bar;
      bar = 42;
    }"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}

#[test]
fn rec_nested_in_attrset_triggers() {
    let registry = make_registry();
    let src = r#"{
      foo = rec {
        bar = 42;
      };
    }"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert_eq!(reports[0].code, 100);
}
