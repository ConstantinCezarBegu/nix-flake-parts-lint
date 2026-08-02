//! Integration test for rule 109: no-firewall-disable
//!
//! Checks that `networking.firewall.enable = false` triggers the no-firewall-disable lint rule.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register(Box::new(nix_lint_rules::NoFirewallDisable::new()));
    registry
}

#[test]
fn firewall_disabled_triggers() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.networking.firewall.enable = false;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 109));
}

#[test]
fn firewall_enabled_no_trigger() {
    let registry = make_registry();
    let src = r#"{ lib, ... }:
{
  options.foo = lib.mkOption { type = lib.types.bool; };
  config.networking.firewall.enable = true;
}"#;
    let reports = nix_lint_core::lint_file(&registry, src).unwrap();
    assert!(reports.is_empty());
}
