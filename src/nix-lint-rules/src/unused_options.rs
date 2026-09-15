use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct UnusedOptions;

impl UnusedOptions {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnusedOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for UnusedOptions {
    fn code(&self) -> u32 {
        125
    }
    fn name(&self) -> &'static str {
        "unused-options"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Options defined in this file are not used within the same file."
    }

    fn validate_file(&self, _path: &Path, content: &str) -> Option<FileLevelReport> {
        // Match options definitions: options.path = lib.mkOption { ... }
        let options_re = Regex::new(r"options\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=").unwrap();

        // Collect all defined option paths
        let defined_options: Vec<String> = options_re
            .captures_iter(content)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        if defined_options.is_empty() {
            return None;
        }

        // Find unused options - options defined but not referenced in config within same file
        let unused: Vec<&str> = defined_options
            .iter()
            .filter(|opt_path| {
                // Use literal dot matching (not regex) for contains()
                let config_pattern = format!("config.{}", opt_path);
                !content.contains(&config_pattern)
            })
            .map(|s| s.as_str())
            .collect();

        eprintln!("DEBUG: unused = {:?}", unused);

        if unused.is_empty() {
            return None;
        }

        Some(FileLevelReport {
            file: _path.to_string_lossy().into_owned(),
            message: format!("Options not used within file: {}", unused.join(", ")),
            note: self.note(),
            code: self.code(),
            severity: self.severity(),
        })
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
    fn test_option_used_in_config_ok() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
          config.foo.bar = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(
            report.is_none(),
            "foo.bar should be considered used"
        );
    }

    #[test]
    fn test_option_not_used_in_config_report() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(
            report.is_some(),
            "foo.bar should be considered unused"
        );
        assert!(
            report.unwrap().message.contains("foo.bar"),
            "Message should mention unused option"
        );
    }

    #[test]
    fn test_nested_option_used_ok() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.nix.server.vpnHost = lib.mkOption { type = lib.types.str; };
          config.nix.server.vpnHost = "test";
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none(), "nix.server.vpnHost should be considered used");
    }

    #[test]
    fn test_multiple_options_one_unused() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
          options.baz.qux = lib.mkOption { type = lib.types.bool; };
          config.foo.bar = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some(), "baz.qux should be considered unused");
        assert!(
            report.unwrap().message.contains("baz.qux"),
            "Message should mention baz.qux as unused"
        );
    }

    #[test]
    fn test_no_options_no_report() {
        let rule = UnusedOptions::new();
        let content = r#"{ pkgs }: {
          packages.myPackage = pkgs.hello;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none(), "should not report when no options defined");
    }

    #[test]
    fn test_deeply_nested_unused() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.wayland.sway.config.workspace-move-focus = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some(), "deeply nested option should be considered unused");
        assert!(
            report.unwrap().message.contains("wayland.sway.config.workspace-move-focus"),
            "Should mention the full option path"
        );
    }

    #[test]
    fn test_darwin_server_options_unused() {
        let rule = UnusedOptions::new();
        let content = r#"{ lib }: {
          options.darwin.nix.server.vpnHost = lib.mkOption { type = lib.types.str; };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some(), "darwin.nix.server.vpnHost should be considered unused");
        assert!(
            report.unwrap().message.contains("darwin.nix.server.vpnHost"),
            "Should mention the full option path"
        );
    }
}
