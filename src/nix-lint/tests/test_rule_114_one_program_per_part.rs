//! Integration test for rule 114: one-program-per-part
//!
//! Checks that multiple flake modules in one file triggers the one-program-per-part lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::OneProgramPerPart::new()));
    registry
}

#[test]
fn multiple_modules_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }: {
  flake.modules.myModule.config = { lib, ... }: {};
  flake.modules.otherModule.options = { lib, ... }: {};
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake.nix"),
        src,
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 114));
}

#[test]
fn single_module_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }: {
  flake.modules.myModule.config = { lib, ... }: {};
}"#;
    let reports = registry.validate_file(
        &std::path::PathBuf::from("flake.nix"),
        src,
    );
    assert!(reports.is_empty());
}
