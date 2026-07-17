use std::path::Path;

use nix_lint_core::{FileLevelRule, FileLevelReport, Severity};
use regex::Regex;

pub struct RequireAssertions;

impl RequireAssertions {
    pub fn new() -> Self { Self }
}

impl FileLevelRule for RequireAssertions {
    fn code(&self) -> u32 { 116 }
    fn name(&self) -> &'static str { "require-assertions" }
    fn severity(&self) -> Severity { Severity::Warn }
    fn note(&self) -> &'static str { "Module defines options but has no assertions." }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let options_re = Regex::new(r"options\.\w+\.\w+").unwrap();
        if !options_re.is_match(content) { return None; }

        let has_assertions = Regex::new(r"assertions\s*=").unwrap();
        let has_assert_stmt = Regex::new(r"\bassert\s+").unwrap();

        if has_assertions.is_match(content) || has_assert_stmt.is_match(content) {
            return None;
        }

        Some(FileLevelReport {
            file: path.to_string_lossy().into_owned(),
            message: "Module defines options but has no assertions. Add an 'assertions = [...]' block or 'assert' statements to validate option values.".into(),
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
    fn test_require_assertions_no_options_no_report() {
        let rule = RequireAssertions::new();
        let content = r#"{ config, lib, ... }: {
          config.foo = "bar";
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_assertions_options_no_assertions_report() {
        let rule = RequireAssertions::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.foo = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.code, 116);
        assert_eq!(report.severity, Severity::Warn);
    }

    #[test]
    fn test_require_assertions_options_with_assertions_block_no_report() {
        let rule = RequireAssertions::new();
        let content = r#"{ config, lib, ... }: {
          options.foo = lib.mkOption { type = lib.types.bool; };
          assertions = [
            { condition = config.foo; }
          ];
          config.foo = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_require_assertions_options_with_assert_stmt_no_report() {
        let rule = RequireAssertions::new();
        let content = r#"{ config, lib, ... }:
  assert config.foo;
  {
    options.foo = lib.mkOption { type = lib.types.bool; };
    config.foo = true;
  }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }
}
