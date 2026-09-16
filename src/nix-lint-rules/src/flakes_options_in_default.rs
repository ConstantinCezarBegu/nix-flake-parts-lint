use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct FlakesOptionsInDefaultOrHostsOption;

impl FlakesOptionsInDefaultOrHostsOption {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlakesOptionsInDefaultOrHostsOption {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for FlakesOptionsInDefaultOrHostsOption {
    fn code(&self) -> u32 {
        121
    }
    fn name(&self) -> &'static str {
        "flakes-options-in-default-or-hosts-option"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Every config.X or options.X namespace must be in its own X-option.nix file. default.nix is allowed for shared configs."
    }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let path_str = path.to_string_lossy();

        // Only apply to files inside hosts/ directories
        if !path_str.contains("/hosts/") && !path_str.starts_with("hosts/") {
            return None;
        }

        // Check if it's a default.nix file (valid - shared config for directory)
        if path.file_name().is_some_and(|n| n == "default.nix") {
            return None;
        }

        // Match both config.X and options.X namespaces
        let config_re = Regex::new(r"\b(config|options)\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=").unwrap();
        if !config_re.is_match(content) {
            return None;
        }

        for cap in config_re.captures_iter(content) {
            let full_match = cap.get(2)?.as_str();
            let ns = full_match.split('.').next()?;
            let qualifier = cap.get(1)?.as_str();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let expected = format!("{}-option.nix", ns);
                if file_name == expected {
                    continue;
                }
                return Some(FileLevelReport {
                    file: path_str.into_owned(),
                    message: format!(
                        "Defines {}.{} but file is '{}', expected '{}'",
                        qualifier, ns, file_name, expected
                    ),
                    note: self.note(),
                    code: self.code(),
                    severity: self.severity(),
                });
            }
        }

        None
    }

    fn validate_project(&self, _files: &[(String, String)]) -> Vec<FileLevelReport> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/test/{}", name))
    }

    #[test]
    fn test_default_nix_valid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("default.nix"), content);
        assert!(report.is_none(), "default.nix should be valid");
    }

    #[test]
    fn test_deeply_nested_default_nix_valid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.wayland.sway = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(
            &make_path("wayland/sway/controls/default.nix"),
            content,
        );
        assert!(report.is_none(), "deeply nested default.nix should be valid");
    }

    #[test]
    fn test_git_option_nix_valid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.git.autocrlf = lib.mkOption { type = lib.types.str; };
        }"#;
        let report = rule.validate_file(&make_path("git-option.nix"), content);
        assert!(report.is_none(), "git-option.nix should be valid");
    }

    #[test]
    fn test_hosts_option_nix_valid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("hosts/myService-option.nix"), content);
        assert!(
            report.is_none(),
            "hosts/myService-option.nix should be valid"
        );
    }

    #[test]
    fn test_hosts_config_nix_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("hosts/config.nix"), content);
        assert!(report.is_some(), "hosts/config.nix should be invalid");
        assert!(
            report.unwrap().message.contains("myService-option.nix"),
            "Should suggest myService-option.nix"
        );
    }

    #[test]
    fn test_config_pattern_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ config }: {
          config.myService.foo = true;
        }"#;
        let report = rule.validate_file(&make_path("hosts/config.nix"), content);
        assert!(report.is_some(), "hosts/config.nix should flag config.myService");
        assert!(
            report.unwrap().message.contains("myService-option.nix"),
            "Should suggest myService-option.nix"
        );
    }

    #[test]
    fn test_git_nix_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.git.autocrlf = lib.mkOption { type = lib.types.str; };
        }"#;
        let report = rule.validate_file(&make_path("hosts/git.nix"), content);
        assert!(report.is_some(), "hosts/git.nix should be invalid");
        assert!(
            report.unwrap().message.contains("git-option.nix"),
            "Should suggest git-option.nix"
        );
    }

    #[test]
    fn test_no_options_no_report() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ pkgs }: {
          packages.myPackage = pkgs.hello;
        }"#;
        let report = rule.validate_file(&make_path("default.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_hosts_ssh_nix_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.programs.ssh.knownHosts = {};
        }"#;
        let report = rule.validate_file(&make_path("hosts/ssh.nix"), content);
        assert!(report.is_some(), "hosts/ssh.nix should be invalid");
    }

    #[test]
    fn test_hosts_secrets_server_default_nix_valid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.secretspecToml = lib.mkOption { type = lib.types.path; };
        }"#;
        let report = rule.validate_file(
            &make_path("hosts/secrets/server/default.nix"),
            content,
        );
        assert!(
            report.is_none(),
            "hosts/secrets/server/default.nix should be valid"
        );
    }

    #[test]
    fn test_sway_svalboard_single_file_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.wayland.sway.svalboard.workspace-move-focus = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(
            &make_path("hosts/wayland/sway/controls/svalboard/workspace-move-focus.nix"),
            content,
        );
        assert!(report.is_some(), "workspace-move-focus.nix should be invalid");
        assert!(
            report.unwrap().message.contains("wayland-option.nix"),
            "Should suggest wayland-option.nix (top-level namespace)"
        );
    }

    #[test]
    fn test_kernel_nix_invalid() {
        let rule = FlakesOptionsInDefaultOrHostsOption::new();
        let content = r#"{ lib }: {
          options.boot.kernelParams = [ "ttm.pages_limit=32505856" ];
        }"#;
        let report = rule.validate_file(&make_path("hosts/boot.nix"), content);
        assert!(report.is_some(), "hosts/boot.nix should be invalid");
    }
}
