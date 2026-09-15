use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct OptionNixRestrictedToHosts;

impl OptionNixRestrictedToHosts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for OptionNixRestrictedToHosts {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for OptionNixRestrictedToHosts {
    fn code(&self) -> u32 {
        124
    }
    fn name(&self) -> &'static str {
        "option-nix-restricted-to-hosts"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "X-option.nix files are only allowed inside hosts/ directories."
    }

    fn validate_file(&self, path: &Path, _content: &str) -> Option<FileLevelReport> {
        let path_str = path.to_string_lossy();

        // Only check files named *-option.nix
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with("-option.nix") => n,
            _ => return None,
        };

        // Check if path contains /hosts/ or starts with hosts/
        if path_str.contains("/hosts/") || path_str.starts_with("hosts/") {
            return None;
        }

        Some(FileLevelReport {
            file: path_str.into_owned(),
            message: format!(
                "{file_name} found outside hosts/ (note: *-option.nix files are only allowed inside hosts/ directories)"
            ),
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
    fn test_option_nix_in_hosts_ok() {
        let rule = OptionNixRestrictedToHosts::new();
        let report = rule.validate_file(&make_path("hosts/myService-option.nix"), "");
        assert!(
            report.is_none(),
            "hosts/myService-option.nix should be valid"
        );
    }

    #[test]
    fn test_option_nix_outside_hosts_report() {
        let rule = OptionNixRestrictedToHosts::new();
        let report = rule.validate_file(&make_path("services/myService-option.nix"), "");
        assert!(report.is_some(), "services/myService-option.nix should be invalid");
        assert!(
            report.unwrap().message.contains("outside hosts/"),
            "Message should mention outside hosts/"
        );
    }

    #[test]
    fn test_non_option_file_no_report() {
        let rule = OptionNixRestrictedToHosts::new();
        let report = rule.validate_file(&make_path("services/default.nix"), "");
        assert!(report.is_none(), "default.nix should not trigger");
    }

    #[test]
    fn test_deeply_nested_in_hosts_ok() {
        let rule = OptionNixRestrictedToHosts::new();
        let report = rule.validate_file(
            &make_path("hosts/wayland/sway/sway-option.nix"),
            "",
        );
        assert!(
            report.is_none(),
            "deeply nested option file in hosts/ should be valid"
        );
    }
}
