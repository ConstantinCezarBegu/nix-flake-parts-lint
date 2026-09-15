use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct FlakesDuplicateOptionsAcrossSiblings;

impl FlakesDuplicateOptionsAcrossSiblings {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FlakesDuplicateOptionsAcrossSiblings {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for FlakesDuplicateOptionsAcrossSiblings {
    fn code(&self) -> u32 {
        122
    }
    fn name(&self) -> &'static str {
        "flakes-duplicate-options-across-siblings"
    }
    fn severity(&self) -> Severity {
        Severity::Hint
    }
    fn note(&self) -> &'static str {
        "Multiple sibling files define the same option namespace — consider consolidating into a parent default.nix."
    }

    fn validate_file(&self, _path: &Path, _content: &str) -> Option<FileLevelReport> {
        None
    }

    fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport> {
        // Group files by parent directory
        let mut dir_files: BTreeMap<&str, Vec<(&str, String)>> = BTreeMap::new();
        for (path, content) in files {
            let p = Path::new(path);
            if let Some(parent) = p.parent().and_then(|d| d.to_str()) {
                // Collect top-level option namespaces from this file
                let options_re = Regex::new(r"\boptions\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=").unwrap();
                let namespaces: BTreeSet<String> = options_re
                    .captures_iter(content)
                    .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
                    .collect();
                if !namespaces.is_empty() {
                    dir_files.entry(parent).or_default().push((path, content.to_string()));
                }
            }
        }

        // For each directory, find shared namespaces across files
        let mut reports = Vec::new();
        for (dir, file_entries) in &dir_files {
            if file_entries.len() < 2 {
                continue;
            }

            // Build a map: namespace -> set of files defining it
            let mut ns_files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (path, content) in file_entries {
                let options_re = Regex::new(r"\boptions\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=").unwrap();
                for cap in options_re.captures_iter(content) {
                    if let Some(ns_match) = cap.get(1) {
                        let full_ns = ns_match.as_str();
                        let top_ns = full_ns.split('.').next().unwrap_or(full_ns);
                        ns_files
                            .entry(top_ns.to_string())
                            .or_default()
                            .insert(path.to_string());
                    }
                }
            }

            // Report namespaces that appear in multiple files
            for (ns, paths) in &ns_files {
                if paths.len() >= 2 {
                    reports.push(FileLevelReport {
                        file: dir.to_string(),
                        message: format!(
                            "Shared option '{}' defined in {} files: {}",
                            ns,
                            paths.len(),
                            paths.iter().cloned().collect::<Vec<_>>().join(", ")
                        ),
                        note: self.note(),
                        code: self.code(),
                        severity: self.severity(),
                    });
                }
            }
        }

        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_files(data: &[(&str, &str)]) -> Vec<(String, String)> {
        data.iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    #[test]
    fn test_sibling_darwin_options_detected() {
        let rule = FlakesDuplicateOptionsAcrossSiblings::new();
        let files = make_files(&[
            (
                "hosts/rewind.nix",
                r#"{ lib }: {
                  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
            (
                "hosts/optimus.nix",
                r#"{ lib }: {
                  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
        ]);
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.code == 122));
        assert!(reports.iter().any(|r| r.message.contains("darwin")));
    }

    #[test]
    fn test_sibling_git_options_detected() {
        let rule = FlakesDuplicateOptionsAcrossSiblings::new();
        let files = make_files(&[
            (
                "development/git.nix",
                r#"{ lib }: {
                  options.git.autocrlf = lib.mkOption { type = lib.types.str; };
                }"#,
            ),
            (
                "development/lfs.nix",
                r#"{ lib }: {
                  options.git.lfs = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
        ]);
        let reports = rule.validate_project(&files);
        assert!(!reports.is_empty());
        assert!(reports.iter().any(|r| r.message.contains("git")));
    }

    #[test]
    fn test_no_shared_options_no_report() {
        let rule = FlakesDuplicateOptionsAcrossSiblings::new();
        let files = make_files(&[
            (
                "services/git.nix",
                r#"{ lib }: {
                  options.git = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
            (
                "services/mpv.nix",
                r#"{ lib }: {
                  options.mpv = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
        ]);
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_single_file_no_report() {
        let rule = FlakesDuplicateOptionsAcrossSiblings::new();
        let files = make_files(&[(
            "hosts/rewind/default.nix",
            r#"{ lib }: {
              options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
            }"#,
        )]);
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }

    #[test]
    fn test_different_dirs_no_cross_report() {
        let rule = FlakesDuplicateOptionsAcrossSiblings::new();
        let files = make_files(&[
            (
                "hosts/rewind/default.nix",
                r#"{ lib }: {
                  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
            (
                "nixos/default.nix",
                r#"{ lib }: {
                  options.darwin.nix.server = lib.mkOption { type = lib.types.bool; };
                }"#,
            ),
        ]);
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty());
    }
}
