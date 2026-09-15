//! Integration test for rule 123: secrets-in-hosts-only

use nix_lint_core::LintRegistry;

fn make_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();
    registry.register_file_level(Box::new(nix_lint_rules::SecretsInHostsOnly::new()));
    registry
}

#[test]
fn secretspec_outside_hosts_warning() {
    let registry = make_registry();
    let files = vec![(
        "development/ai/opencode.nix".to_string(),
        r#"{ pkgs, secretspec, secretspecToml }:
    secretspec
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected secretspec warning outside hosts/");
    assert!(
        reports.iter().any(|r| r.code == 123),
        "expected rule 123 violation"
    );
    assert!(
        reports.iter().any(|r| r.message.contains("secretspec")),
        "expected 'secretspec' in message"
    );
    assert!(
        reports.iter().all(|r| r.severity == nix_lint_core::Severity::Warn),
        "expected Warn severity"
    );
}

#[test]
fn mkSecretCmd_outside_hosts_warning() {
    let registry = make_registry();
    let files = vec![(
        "services/bw.nix".to_string(),
        r#"{ config, mkSecretCmd }:
    ${mkSecretCmd { key = "api-key"; }}
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected mkSecretCmd warning outside hosts/");
    assert!(
        reports.iter().any(|r| r.code == 123),
        "expected rule 123 violation"
    );
}

#[test]
fn secretspec_in_hosts_no_warning() {
    let registry = make_registry();
    let files = vec![(
        "hosts/megatron/ssh.nix".to_string(),
        r#"{ config, lib, mkSecretCmd, pkgs, secretspec, ... }:
    secrets = import ../secrets/mkSecret.nix { inherit pkgs secretspec; };
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(reports.is_empty(), "expected no warning for secrets in hosts/");
}

#[test]
fn secretspecToml_in_nixos_warning() {
    let registry = make_registry();
    let files = vec![(
        "nixos/default.nix".to_string(),
        r#"{ secretspecToml }:
    { config.secretspecToml = secretspecToml; }
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected secretspecToml warning outside hosts/");
    assert!(
        reports.iter().any(|r| r.code == 123),
        "expected rule 123 violation"
    );
}

#[test]
fn bwSession_in_darwin_warning() {
    let registry = make_registry();
    let files = vec![(
        "darwin/default.nix".to_string(),
        r#"{ bwSession }:
    { config = { }; }
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected bwSession warning outside hosts/");
    assert!(
        reports.iter().any(|r| r.code == 123),
        "expected rule 123 violation"
    );
}

#[test]
fn options_secrets_outside_hosts_warning() {
    let registry = make_registry();
    let files = vec![(
        "nixos/secretspec.nix".to_string(),
        r#"{ secretspec }:
    options.secrets = {
        vpnHost = lib.mkOption { type = lib.types.str; };
    };
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(!reports.is_empty(), "expected options.secrets warning outside hosts/");
    assert!(
        reports.iter().any(|r| r.code == 123),
        "expected rule 123 violation"
    );
}

#[test]
fn no_secrets_no_report() {
    let registry = make_registry();
    let files = vec![(
        "services/git.nix".to_string(),
        r#"{ lib }:
    options.git = lib.mkOption { type = lib.types.bool; };
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(
        reports.is_empty(),
        "expected no report when no secrets used"
    );
}

#[test]
fn secrets_in_hosts_secrets_dir_ok() {
    let registry = make_registry();
    let files = vec![(
        "hosts/secrets/mkSecret.nix".to_string(),
        r#"{ secretspec, secretspecToml, bwSession, mkSecretCmd, pkgs, ... }:
    mkSecretCmd
"#
            .to_string(),
    )];
    let reports = registry.validate_project(&files);
    assert!(
        reports.is_empty(),
        "expected no report for secrets in hosts/secrets/"
    );
}
