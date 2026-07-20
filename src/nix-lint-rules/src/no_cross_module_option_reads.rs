use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

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
        let options_re = Regex::new(r"\boptions\.([a-zA-Z_]\w*)").unwrap();
        let declared: Vec<&str> = options_re
            .captures_iter(content)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str())
            .collect();
        if declared.is_empty() {
            return None;
        }
        let declared_set: std::collections::HashSet<&str> = declared.iter().copied().collect();

        // Match top-level config reads: <non-identifier/dot>config.<namespace>.
        let config_read_re = Regex::new(r"([^a-zA-Z0-9_.])config\.([a-zA-Z_]\w*)\.").unwrap();
        for cap in config_read_re.captures_iter(content) {
            let ns = cap.get(2)?.as_str();
            if !declared_set.contains(ns) {
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

        // Check assertions: assert ... <non-dot>config.<namespace>.
        let assert_config_re =
            Regex::new(r"assert\s+.*?([^a-zA-Z0-9_.])config\.([a-zA-Z_]\w*)\.").unwrap();
        for cap in assert_config_re.captures_iter(content) {
            let ns = cap.get(2)?.as_str();
            if !declared_set.contains(ns) {
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
    #![allow(dead_code)]
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
}
