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

        // Check if an option is referenced in config within the file
        fn is_option_used(option: &str, content: &str) -> bool {
            let parts: Vec<&str> = option.split('.').collect();

            // Check direct access: config.foo.bar
            if content.contains(&format!("config.{}", option)) {
                return true;
            }

            // Check string-keyed access: config."foo.bar" or config."foo".bar
            if content.contains(&format!("config.\"{}\"", option)) {
                return true;
            }

            // Check string-keyed access to first segment: config."foo".bar.baz
            if parts.len() >= 2 {
                let first = parts[0];
                let rest = parts[1..].join(".");
                let quoted_pattern = format!("config.\"{}\".{}", first, rest);
                if content.contains(&quoted_pattern) {
                    return true;
                }
            }

            false
        }

        // Check if an option definition is inside a non-NixOS context (local data structures)
        fn is_local_option(option: &str, content: &str) -> bool {
            // Check if this option appears inside a let-binding data structure (not in config = { ... })
            // Pattern: localBundles.options.X or similar data structures
            let pattern = format!("options.{}", option);
            let lines: Vec<&str> = content.lines().collect();
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.starts_with(&pattern) || trimmed.contains(&format!(".{}", option)) {
                    // Check if this line is inside a data structure (preceded by let { or inside localBundles)
                    // Look backwards from this line to find if we're inside a let { block
                    let before = &content[..content.find(*line).unwrap_or(0)];
                    // Check if it's a Nixvim keymap option (options.silent inside keymaps array)
                    if option == "silent" || option.starts_with("silent.") {
                        if before.contains("keymaps") || before.contains("programs.nixvim") {
                            return true;
                        }
                    }
                    // If preceded by "let" and not inside "config =", it's likely a local data structure
                    if before.contains("let") && !before.ends_with("config =") && !before.ends_with("config=") {
                        // Check if it's inside a local data structure (before config =)
                        if before.find("config =").is_none() && before.find("config=").is_none() {
                            return true;
                        }
                    }
                }
            }
            false
        }

        let unused: Vec<&str> = defined_options
            .iter()
            .filter(|opt| {
                // Skip options that are in local data structures (not NixOS module options)
                if is_local_option(opt, content) {
                    return false;
                }
                !is_option_used(opt, content)
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
