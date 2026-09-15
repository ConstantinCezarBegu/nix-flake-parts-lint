//! Integration test for rule 124: option-nix-restricted-to-hosts
//!
//! *-option.nix files should only exist inside hosts/ directories.

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::OptionNixRestrictedToHosts::new()));
    registry
}

#[test]
fn option_nix_in_hosts_ok() {
    let registry = make_registry();
    let reports = registry.validate_file(
        &std::path::PathBuf::from("hosts/myService-option.nix"),
        "",
    );
    assert!(reports.is_empty(), "hosts/myService-option.nix should be valid");
}

#[test]
fn option_nix_outside_hosts_report() {
    let registry = make_registry();
    let reports = registry.validate_file(
        &std::path::PathBuf::from("services/myService-option.nix"),
        "",
    );
    assert!(!reports.is_empty());
    assert!(reports.iter().any(|r| r.code == 124));
    assert!(reports.iter().any(|r| r.message.contains("outside hosts/")));
}

#[test]
fn non_option_file_no_report() {
    let registry = make_registry();
    let reports = registry.validate_file(
        &std::path::PathBuf::from("services/default.nix"),
        "",
    );
    assert!(reports.is_empty());
}

#[test]
fn deeply_nested_in_hosts_ok() {
    let registry = make_registry();
    let reports = registry.validate_file(
        &std::path::PathBuf::from("hosts/wayland/sway/sway-option.nix"),
        "",
    );
    assert!(reports.is_empty(), "deeply nested option file in hosts/ should be valid");
}
