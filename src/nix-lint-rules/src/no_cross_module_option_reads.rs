use std::collections::HashSet;
use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

use crate::builtin_namespaces::{BUILTIN_OPTIONS, in_file_name};

pub struct NoCrossModuleOptionReads;

impl NoCrossModuleOptionReads {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoCrossModuleOptionReads {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for NoCrossModuleOptionReads {
    fn code(&self) -> u32 {
        118
    }
    fn name(&self) -> &'static str {
        "no-cross-module-option-reads"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Module reads config from namespace not declared as options in this file."
    }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        // Nix bare identifiers allow hyphens (e.g. `home-manager`), unlike
        // regex's `\w`. Without `-` in the class, "config.home-manager..."
        // truncates to "home" at the hyphen and never reaches the
        // "home-manager" entry in BUILTIN_OPTIONS below.
        let options_re = Regex::new(r"\boptions\.([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap();
        let declared: Vec<&str> = options_re
            .captures_iter(content)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str())
            .collect();
        if declared.is_empty() {
            return None;
        }
        let declared_set: HashSet<&str> = declared.iter().copied().collect();
        let builtin_set: HashSet<&str> = BUILTIN_OPTIONS.iter().copied().collect();

        // Match all config.X entry points where X is a namespace.
        // Matches config.<identifier> followed by non-identifier chars or end of string.
        // Greedy + explicit character class (including '-') so the whole
        // identifier is captured in one go; no trailing \b needed since \b's
        // own word-boundary definition doesn't know about hyphens either.
        let config_read_re = Regex::new(r"\bconfig\.([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap();
        for cap in config_read_re.captures_iter(content) {
            let ns = cap.get(1)?.as_str();
            if in_file_name(content, cap.get(0)?.start()) {
                continue;
            }
            let match_end = cap.get(1).unwrap().end();
            let next_char = content[match_end..].chars().next();
            // Skip if the namespace continues (next char is a valid identifier char)
            if next_char.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                continue;
            }
            if !declared_set.contains(ns) && !builtin_set.contains(ns) {
                return Some(FileLevelReport {
                    file: path.to_string_lossy().into_owned(),
                    message: format!(
                        "Module reads config.{} but does not declare options.{} in this file.",
                        ns, ns
                    ),
                    note: self.note(),
                    code: self.code(),
                    severity: self.severity(),
                });
            }
        }

        // Check assertions: assert followed by config.X entry point.
        let assert_config_re =
            Regex::new(r"assert\s+.*?\bconfig\.([a-zA-Z_][a-zA-Z0-9_-]*)").unwrap();
        for cap in assert_config_re.captures_iter(content) {
            let ns = cap.get(1)?.as_str();
            let match_end = cap.get(1).unwrap().end();
            let next_char = content[match_end..].chars().next();
            // Skip if the namespace continues (next char is a valid identifier char)
            if next_char.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                continue;
            }
            if !declared_set.contains(ns) && !builtin_set.contains(ns) {
                return Some(FileLevelReport {
                    file: path.to_string_lossy().into_owned(),
                    message: format!(
                        "Module asserts on config.{} but does not declare options.{} in this file.",
                        ns, ns
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
    fn test_cross_module_read_same_namespace_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.myService.foo;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_cross_module_read_different_namespace_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.otherService.baz;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.code, 118);
        assert_eq!(report.severity, Severity::Error);
        assert!(report.message.contains("otherService"));
    }

    #[test]
    fn test_cross_module_assert_different_namespace_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          assert config.otherService.enabled;
          {
            config.myService.bar = true;
          }
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        assert!(report.unwrap().message.contains("otherService"));
    }

    #[test]
    fn test_no_options_declared_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          config.foo = config.bar;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_multiple_declared_namespaces_same_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.serviceA.foo = lib.mkOption { type = lib.types.bool; };
          options.serviceB.bar = lib.mkOption { type = lib.types.int; };
          config.serviceA.foo = config.serviceB.bar > 0;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_nested_path_same_namespace_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo.bar = lib.mkOption { type = lib.types.bool; };
          config.myService.foo.bar = config.myService.foo.baz;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_hyphenated_home_manager_read_no_report() {
        // Regression: "config.home-manager.users..." must resolve the full
        // hyphenated identifier "home-manager" against BUILTIN_OPTIONS, not
        // truncate at the hyphen to "home" (which isn't itself a builtin).
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.home-manager.users.someuser.home.foo;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_nix_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.nixSetting = config.nix.settings.auto-optimise-store;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_services_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.package = lib.mkOption { type = lib.types.package; };
          config.myService.existingPackage = config.services.nginx.package;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_flake_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.desc = lib.mkOption { type = lib.types.str; };
          config.myService.desc = config.flake.description;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_darwin_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.autologin = lib.mkOption { type = lib.types.bool; };
          config.myService.autologin = config.darwin.enableAutologin;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_programs_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.gitEnable = lib.mkOption { type = lib.types.bool; };
          config.myService.gitEnable = config.programs.git.enable;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_user_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.users = lib.mkOption { type = lib.types.listOf lib.types.str; };
          config.myService.users = config.users.users."myuser".name;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_assertion_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          assert config.nix.settings.auto-optimise-store;
          {
            config.myService.bar = true;
          }
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_non_builtin_undeclared_read_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.undeclaredThing.baz;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert!(report.message.contains("undeclaredThing"));
    }

    #[test]
    fn test_multiple_builtin_reads_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.nix = config.nix.settings.auto-optimise-store;
          config.myService.nginx = config.services.nginx.enable;
          config.myService.desc = config.flake.description;
          config.myService.darwin = config.darwin.enableAutologin;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_and_declared_mixed_read_no_report() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.myService.foo && config.nix.settings.auto-optimise-store;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_read_at_start_of_line() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
config.services.nginx.enable = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_read_at_start_of_file() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
config.myService.bar = config.services.nginx.enable;
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_standalone_expression_no_trailing_dot() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  let x = config.services;
  in config.myService.bar = x;
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_assignment_target_no_trailing_dot() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.services = { nginx.enable = true; };
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_in_array_no_trailing_dot() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.packages = [ config.services ];
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_undeclared_standalone_expression() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  let x = config.otherService;
  in config.myService.bar = x;
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        assert!(report.unwrap().message.contains("otherService"));
    }

    #[test]
    fn test_config_in_comparison_no_trailing_dot() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.bar = config.services == { };
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_myconfig_not_matched() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  config.myService.bar = myConfig.services.enable;
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_in_assert_multiline() {
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
  options.myService.foo = lib.mkOption { type = lib.types.bool; };
  assert
    config.otherService.enabled;
  {
    config.myService.bar = true;
  }
}"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        assert!(report.unwrap().message.contains("otherService"));
    }

    #[test]
    fn test_config_inside_file_name_no_report() {
        // "config.yaml" here is part of a file name in a string, not the
        // config.yaml namespace.
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.port = lib.mkOption { type = lib.types.port; };
          config.xdg.configFile."my-app/config.yaml".text = "port: ${toString config.myService.port}";
        }"#;
        assert!(
            rule.validate_file(&make_path("test.nix"), content)
                .is_none()
        );
    }

    #[test]
    fn test_module_system_lib_namespace_no_report() {
        // config.lib is the module system's own option (NixOS and
        // home-manager), where e.g. Stylix puts its colors.
        let rule = NoCrossModuleOptionReads::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.theme = lib.mkOption { type = lib.types.str; };
          config.myService.theme = config.lib.stylix.colors.base00;
        }"#;
        assert!(
            rule.validate_file(&make_path("test.nix"), content)
                .is_none()
        );
    }
}
