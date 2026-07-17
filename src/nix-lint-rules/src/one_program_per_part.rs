use std::path::Path;

use nix_lint_core::{FileLevelRule, FileLevelReport, Severity};
use regex::Regex;

pub struct OneProgramPerPart;

impl OneProgramPerPart {
    pub fn new() -> Self { Self }
}

impl FileLevelRule for OneProgramPerPart {
    fn code(&self) -> u32 { 114 }
    fn name(&self) -> &'static str { "one-program-per-part" }
    fn severity(&self) -> Severity { Severity::Error }
    fn note(&self) -> &'static str { "Multiple flake modules in one file." }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let re = Regex::new(r"flake\.modules\.(\w+)\.(\S+)").unwrap();
        let names: Vec<&str> = re.captures_iter(content)
            .filter_map(|c| c.get(2))
            .map(|m| m.as_str())
            .collect();
        if names.len() <= 1 { return None; }
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        if unique.len() > 1 {
            let cleaned: Vec<_> = unique.iter().map(|s| {
                s.trim_start_matches(|c: char| c == '.' || c == '(' || c == '|')
            }).collect();
            Some(FileLevelReport {
                file: path.to_string_lossy().into_owned(),
                message: format!("Found {} different flake modules in one file: {}. Each file should define exactly one program/module.", unique.len(), cleaned.join(", ")),
                note: self.note(),
                code: self.code(),
                severity: self.severity(),
            })
        } else { None }
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
    fn test_one_program_per_part_single_module_no_report() {
        let rule = OneProgramPerPart::new();
        let content = r#"{ lib, ... }: {
          flake.modules.myModule = { config, lib, ... }: {
            options.foo = lib.mkOption { type = lib.types.bool; };
          };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_one_program_per_part_two_modules_report() {
        let rule = OneProgramPerPart::new();
        let content = r#"{ lib, ... }: {
          flake.modules.myModule.config = { lib, ... }: {};
          flake.modules.otherModule.config = { lib, ... }: {};
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_one_program_per_part_two_modules_different_suffixes_report() {
        let rule = OneProgramPerPart::new();
        let content = r#"{ lib, ... }: {
          flake.modules.myModule.config = { lib, ... }: {};
          flake.modules.otherModule.options = { lib, ... }: {};
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert_eq!(report.code, 114);
        assert_eq!(report.severity, Severity::Error);
    }
}
