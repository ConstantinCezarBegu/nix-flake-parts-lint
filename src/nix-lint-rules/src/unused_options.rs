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

/// Match options definitions: options.path = lib.mkOption { ... }
fn defined_options(content: &str) -> Vec<String> {
    let options_re =
        Regex::new(r"options\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=")
            .unwrap();
    options_re
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

/// Check if an option is referenced via `config.<option>` (or a string-keyed
/// equivalent) anywhere in the given content.
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

/// Check if an option definition is inside a non-NixOS context (local data
/// structures), based on the file where it's declared.
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
        "Options defined in this file are not used anywhere in the project."
    }

    fn validate_file(&self, _path: &Path, _content: &str) -> Option<FileLevelReport> {
        // Usage can legitimately live in a different file than the
        // declaration (e.g. a small file sets an `internal = true` option
        // that a sibling aggregator file reads), so this rule only makes
        // sense checked project-wide. See validate_project.
        None
    }

    fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport> {
        let mut reports = Vec::new();

        for (path, content) in files {
            let defined = defined_options(content);
            if defined.is_empty() {
                continue;
            }

            let unused: Vec<&str> = defined
                .iter()
                .filter(|opt| {
                    // Skip options that are in local data structures (not NixOS module options)
                    if is_local_option(opt, content) {
                        return false;
                    }
                    !files
                        .iter()
                        .any(|(_, other_content)| is_option_used(opt, other_content))
                })
                .map(|s| s.as_str())
                .collect();

            if !unused.is_empty() {
                reports.push(FileLevelReport {
                    file: path.clone(),
                    message: format!(
                        "Options not used anywhere in the project: {}",
                        unused.join(", ")
                    ),
                    note: self.note(),
                    code: self.code(),
                    severity: self.severity(),
                });
            }
        }

        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(content: &str) -> Vec<FileLevelReport> {
        let rule = UnusedOptions::new();
        rule.validate_project(&[("test.nix".to_string(), content.to_string())])
    }

    fn validate_files(files: &[(&str, &str)]) -> Vec<FileLevelReport> {
        let rule = UnusedOptions::new();
        let owned: Vec<(String, String)> = files
            .iter()
            .map(|(p, c)| (p.to_string(), c.to_string()))
            .collect();
        rule.validate_project(&owned)
    }

    #[test]
    fn test_option_used_in_config_ok() {
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
          config.foo.bar = true;
        }"#;
        let reports = validate(content);
        assert!(reports.is_empty(), "foo.bar should be considered used");
    }

    #[test]
    fn test_option_not_used_in_config_report() {
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
        }"#;
        let reports = validate(content);
        assert_eq!(reports.len(), 1, "foo.bar should be considered unused");
        assert!(
            reports[0].message.contains("foo.bar"),
            "Message should mention unused option"
        );
    }

    #[test]
    fn test_nested_option_used_ok() {
        let content = r#"{ lib }: {
          options.nix.server.vpnHost = lib.mkOption { type = lib.types.str; };
          config.nix.server.vpnHost = "test";
        }"#;
        let reports = validate(content);
        assert!(reports.is_empty(), "nix.server.vpnHost should be considered used");
    }

    #[test]
    fn test_multiple_options_one_unused() {
        let content = r#"{ lib }: {
          options.foo.bar = lib.mkOption { type = lib.types.bool; };
          options.baz.qux = lib.mkOption { type = lib.types.bool; };
          config.foo.bar = true;
        }"#;
        let reports = validate(content);
        assert_eq!(reports.len(), 1, "baz.qux should be considered unused");
        assert!(
            reports[0].message.contains("baz.qux"),
            "Message should mention baz.qux as unused"
        );
    }

    #[test]
    fn test_no_options_no_report() {
        let content = r#"{ pkgs }: {
          packages.myPackage = pkgs.hello;
        }"#;
        let reports = validate(content);
        assert!(reports.is_empty(), "should not report when no options defined");
    }

    #[test]
    fn test_deeply_nested_unused() {
        let content = r#"{ lib }: {
          options.wayland.sway.config.workspace-move-focus = lib.mkOption { type = lib.types.bool; };
        }"#;
        let reports = validate(content);
        assert_eq!(reports.len(), 1, "deeply nested option should be considered unused");
        assert!(
            reports[0].message.contains("wayland.sway.config.workspace-move-focus"),
            "Should mention the full option path"
        );
    }

    #[test]
    fn test_darwin_server_options_unused() {
        let content = r#"{ lib }: {
          options.darwin.nix.server.vpnHost = lib.mkOption { type = lib.types.str; };
        }"#;
        let reports = validate(content);
        assert_eq!(reports.len(), 1, "darwin.nix.server.vpnHost should be considered unused");
        assert!(
            reports[0].message.contains("darwin.nix.server.vpnHost"),
            "Should mention the full option path"
        );
    }

    #[test]
    fn test_option_used_in_sibling_file_ok() {
        let declaring = r#"{ lib, pkgs, ... }: {
          options.wayland.sway.controls.focusOutputScript = lib.mkOption {
            type = lib.types.path;
            internal = true;
          };
          config = {
            wayland.sway.controls.focusOutputScript = pkgs.writeShellScript "x" "";
          };
        }"#;
        let consumer = r#"{ config, ... }: {
          config = {
            bindings."a" = config.wayland.sway.controls.focusOutputScript;
          };
        }"#;
        let reports = validate_files(&[("declare.nix", declaring), ("consume.nix", consumer)]);
        assert!(
            reports.is_empty(),
            "option read in a sibling file should count as used"
        );
    }

    #[test]
    fn test_option_unused_across_project_report() {
        let declaring = r#"{ lib, pkgs, ... }: {
          options.wayland.sway.controls.deadScript = lib.mkOption {
            type = lib.types.path;
            internal = true;
          };
          config = {
            wayland.sway.controls.deadScript = pkgs.writeShellScript "x" "";
          };
        }"#;
        let other = r#"{ config, ... }: {
          config.somethingElse = config.wayland.sway.controls.unrelated;
        }"#;
        let reports = validate_files(&[("declare.nix", declaring), ("other.nix", other)]);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].message.contains("wayland.sway.controls.deadScript"));
    }
}
