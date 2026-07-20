use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct RequireFlakeParts;

impl RequireFlakeParts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequireFlakeParts {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for RequireFlakeParts {
    fn code(&self) -> u32 {
        115
    }
    fn name(&self) -> &'static str {
        "require-flake-parts"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "File is not a flake module and not a subfile of one."
    }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let filename = path.file_name()?.to_str()?;
        if filename == "default.nix" || filename == "flake.nix" {
            return None;
        }

        let path_str = path.to_string_lossy();
        if path_str.contains("/hosts/")
            || path_str.contains("/secrets/")
            || path_str.contains("/packages/")
            || path_str.contains("/test-fixtures/")
            || path_str.contains("/tests/")
            || path_str.contains("test-fixtures/")
            || path_str.contains("tests/")
        {
            return None;
        }

        let has_flake_modules = Regex::new(r"flake\.modules\.").unwrap();
        if has_flake_modules.is_match(content) {
            return None;
        }

        let mut dir = path.parent();
        while let Some(d) = dir {
            let default_nix = d.join("default.nix");
            if default_nix.is_file() {
                if let Ok(parent_content) = std::fs::read_to_string(&default_nix) {
                    if has_flake_modules.is_match(&parent_content) {
                        return None;
                    }
                }
            }
            dir = d.parent();
        }

        Some(FileLevelReport {
            file: path_str.into_owned(),
            message: "File is neither a flake module nor a subfile of one. Every program should be a flake part.".into(),
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
    #![allow(dead_code)]
    use super::*;
    use std::path::PathBuf;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/test/{}", name))
    }

    #[test]
    fn test_require_flake_parts_not_a_module_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{
  # A regular nix file with flake modules
  flake.modules.foo = { lib, ... }: {};
}"#;
        let report = rule.validate_file(&make_path("flake.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_default_nix_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("default.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_hosts_dir_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("hosts/myhost.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_packages_dir_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("packages/myapp.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_secrets_dir_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("secrets/secrets.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_test_fixtures_dir_no_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("test-fixtures/test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_flake_parts_orphan_file_report() {
        let rule = RequireFlakeParts::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
        }"#;
        let report = rule.validate_file(&make_path("orphan.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.code, 115);
        assert_eq!(report.severity, Severity::Error);
    }
}
