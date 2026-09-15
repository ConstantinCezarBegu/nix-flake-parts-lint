//! Integration test for rule 122: flakes-duplicate-options-across-siblings

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(
        nix_lint_rules::FlakesDuplicateOptionsAcrossSiblings::new(),
    ));
    registry
}

#[test]
fn sibling_darwin_options_detected() {
    let registry = make_registry();
    let files = vec![
        (
            "hosts/rewind.nix".to_string(),
            r#"{ lib }: {
  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
        (
            "hosts/optimus.nix".to_string(),
            r#"{ lib }: {
  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
    ];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected shared 'darwin' option report");
    assert!(
        reports.iter().any(|r| r.code == 122),
        "expected rule 122 violation"
    );
    assert!(
        reports.iter().any(|r| r.message.contains("darwin")),
        "expected 'darwin' in message"
    );
}

#[test]
fn sway_svalboard_siblings_detected() {
    let registry = make_registry();
    let files = vec![
        (
            "wayland/sway/controls/svalboard/focus-boundary.nix".to_string(),
            r#"{ lib }: {
  options.sway.bindings = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
        (
            "wayland/sway/controls/svalboard/scratchpad-toggle.nix".to_string(),
            r#"{ lib }: {
  options.sway.bindings = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
    ];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected shared 'sway' option report");
    assert!(
        reports.iter().any(|r| r.message.contains("sway")),
        "expected 'sway' in message"
    );
}

#[test]
fn no_shared_options_no_report() {
    let registry = make_registry();
    let files = vec![
        (
            "services/git.nix".to_string(),
            r#"{ lib }: {
  options.git = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
        (
            "services/mpv.nix".to_string(),
            r#"{ lib }: {
  options.mpv = lib.mkOption { type = lib.types.bool; };
}"#
                .to_string(),
        ),
    ];
    let reports = registry.validate_project(&files);
    assert!(
        reports.is_empty(),
        "expected no report for different namespaces"
    );
}

#[test]
fn single_file_no_report() {
    let registry = make_registry();
    let files = vec![(
        "hosts/rewind.nix".to_string(),
        r#"{ lib }: {
  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
}"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(
        reports.is_empty(),
        "expected no report for single file"
    );
}
