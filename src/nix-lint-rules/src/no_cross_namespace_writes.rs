use std::path::Path;

use nix_lint_core::{FileLevelRule, FileLevelReport, Severity};
use regex::Regex;

pub struct NoCrossNamespaceWrites;

impl NoCrossNamespaceWrites {
    pub fn new() -> Self { Self }
}

impl FileLevelRule for NoCrossNamespaceWrites {
    fn code(&self) -> u32 { 117 }
    fn name(&self) -> &'static str { "no-cross-namespace-writes" }
    fn severity(&self) -> Severity { Severity::Error }
    fn note(&self) -> &'static str { "Module writes to config namespace not declared as options in this file." }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let options_re = Regex::new(r"\boptions\.([a-zA-Z_]\w*)").unwrap();
        let declared: Vec<&str> = options_re.captures_iter(content).filter_map(|c| c.get(1)).map(|m| m.as_str()).collect();
        if declared.is_empty() { return None; }
        let declared_set: std::collections::HashSet<&str> = declared.iter().copied().collect();

        // Match config.<namespace> where it's NOT preceded by identifier or dot chars (nested paths)
        // Uses capture group 1 for context char and group 2 for namespace
        let config_write_re = Regex::new(r"([^a-zA-Z0-9_.])config\.([a-zA-Z_]\w*)(?:\s*=|\.)").unwrap();
        for cap in config_write_re.captures_iter(content) {
            let ns = cap.get(2)?.as_str();
            if !declared_set.contains(ns) {
                return Some(FileLevelReport {
                    file: path.to_string_lossy().into_owned(),
                    message: format!("Module writes to config.{} but does not declare options.{} in this file.", ns, ns),
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
    fn test_cross_namespace_write_same_namespace_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.foo = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_cross_namespace_write_different_namespace_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.otherService.foo = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.code, 117);
        assert_eq!(report.severity, Severity::Error);
        assert!(report.message.contains("otherService"));
    }

    #[test]
    fn test_no_options_declared_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          config.foo = "bar";
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_cross_namespace_write_multiple_namespaces_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.serviceA.foo = lib.mkOption { type = lib.types.bool; };
          config.serviceB.bar = 42;
          config.serviceC.baz = "hello";
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        assert!(report.unwrap().message.contains("serviceB"));
    }

    #[test]
    fn test_nested_config_read_same_namespace_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.myService.foo;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_nested_path_same_namespace_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo.bar = lib.mkOption { type = lib.types.bool; };
          config.myService.foo.bar = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }
}
