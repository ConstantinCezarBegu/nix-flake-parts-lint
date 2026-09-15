use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

/// Detects secrets usage (secretspec, secretspecToml, mkSecretCmd, bwSession,
/// or options.secrets) outside of the hosts/ directory.
pub struct SecretsInHostsOnly;

impl SecretsInHostsOnly {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SecretsInHostsOnly {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for SecretsInHostsOnly {
    fn code(&self) -> u32 {
        123
    }
    fn name(&self) -> &'static str {
        "secrets-in-hosts-only"
    }
    fn severity(&self) -> Severity {
        Severity::Warn
    }
    fn note(&self) -> &'static str {
        "Secrets (secretspec, secretspecToml, mkSecretCmd, bwSession, options.secrets) should only be used within hosts/."
    }

    fn validate_file(&self, _path: &Path, _content: &str) -> Option<FileLevelReport> {
        // This rule only reports at project level, not file level
        None
    }

    fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport> {
        // Check each file for secrets identifiers outside hosts/
        let mut reports = Vec::new();
        for (path_str, content) in files {
            let path = Path::new(path_str);
            let in_hosts = path_str.starts_with("hosts/") || path_str.contains("/hosts/");
            if !in_hosts {
                // Check for secrets-related identifiers
                let secrets_patterns = [
                    r"\bsecretspec\b",
                    r"\bsecretspecToml\b",
                    r"\bmkSecretCmd\b",
                    r"\bbwSession\b",
                    r"\boptions\.secrets\b",
                ];

                for pattern in &secrets_patterns {
                    let re = Regex::new(pattern).unwrap();
                    if re.is_match(content) {
                        let id = pattern
                            .replace(r"\b", "")
                            .replace(r"\boptions\.secrets\b", "options.secrets");
                        reports.push(FileLevelReport {
                            file: path_str.to_string(),
                            message: format!("Secrets identifier '{}' found outside hosts/", id),
                            note: self.note(),
                            code: self.code(),
                            severity: self.severity(),
                        });
                        break; // Only report once per file
                    }
                }
            }
        }
        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file(path: &str, content: &str) -> Vec<(String, String)> {
        vec![(path.to_string(), content.to_string())]
    }

    #[test]
    fn test_secretspec_in_hosts_ok() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "hosts/megatron/ssh.nix",
            r#"{ config, lib, mkSecretCmd, pkgs, secretspec, ... }:
      secrets = import ../secrets/mkSecret.nix { inherit pkgs secretspec; };
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_secretspec_outside_hosts_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "development/ai/opencode.nix",
            r#"{ pkgs, secretspec, secretspecToml }:
    secretspec
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 123));
        assert!(reports.iter().any(|r| r.message.contains("secretspec")));
    }

    #[test]
    fn test_mkSecretCmd_outside_hosts_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "services/bw.nix",
            r#"{ config, secretspec, mkSecretCmd }:
      ${mkSecretCmd { key = "api-key"; }}
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 123));
    }

    #[test]
    fn test_secretspecToml_outside_hosts_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "nixos/default.nix",
            r#"{ secretspecToml }:
    { config.secretspecToml = secretspecToml; }
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 123));
    }

    #[test]
    fn test_bwSession_outside_hosts_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "darwin/default.nix",
            r#"{ bwSession }:
    { config = { }; }
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 123));
    }

    #[test]
    fn test_options_secrets_outside_hosts_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "nixos/secretspec.nix",
            r#"{ secretspec }:
    options.secrets = {
      vpnHost = lib.mkOption { type = lib.types.str; };
    };
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 123));
    }

    #[test]
    fn test_no_secrets_no_report() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "services/git.nix",
            r#"{ lib }:
    options.git = lib.mkOption { type = lib.types.bool; };
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_secrets_in_hosts_secrets_dir_ok() {
        let rule = SecretsInHostsOnly::new();
        let files = make_file(
            "hosts/secrets/mkSecret.nix",
            r#"{ secretspec, secretspecToml, bwSession, mkSecretCmd, pkgs, ... }:
    mkSecretCmd
"#,
        );
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }
}
