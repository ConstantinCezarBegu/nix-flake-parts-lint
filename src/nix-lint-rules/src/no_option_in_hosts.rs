use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct NoOptionInHosts;

impl NoOptionInHosts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoOptionInHosts {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for NoOptionInHosts {
    fn code(&self) -> u32 {
        126
    }
    fn name(&self) -> &'static str {
        "no-option-in-hosts"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Options should be declared in flake namespaces, not inside hosts/ modules."
    }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let path_str = path.to_string_lossy();

        // Only check files inside hosts/ directories
        if !path_str.contains("/hosts/") && !path_str.starts_with("hosts/") {
            return None;
        }

        // Match all option.X and options.X references (not config.X)
        let option_re = Regex::new(r"\boptions?\s*\.\s*([a-zA-Z_]\w*)").unwrap();

        for cap in option_re.captures_iter(content) {
            let ns = cap.get(1)?.as_str();

            // Skip flake-parts built-in options
            if matches!(ns, "flake" | "perSystem" | "system" | "nixpkgs" | "inputs" | "outputs") {
                continue;
            }

            return Some(FileLevelReport {
                file: path_str.into_owned(),
                message: format!(
                    "Found option.{} / options.{} in hosts/ (options should be declared in flake namespaces, not inside hosts/ modules)",
                    ns, ns
                ),
                note: self.note(),
                code: self.code(),
                severity: self.severity(),
            });
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
    fn test_option_in_hosts_reports() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          options.myService.enabled = lib.mkOption { type = lib.types.bool; default = false; };
          config.myService.enabled = true;
        }";
        let report = rule.validate_file(&make_path("hosts/rewind/default.nix"), content);
        assert!(report.is_some(), "option.myService in hosts/ should trigger");
        assert!(report.unwrap().message.contains("myService"));
    }

    #[test]
    fn test_options_in_hosts_reports() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          options.networking.hostName = \"rewind\";
          config.networking.hostName = \"rewind\";
        }";
        let report = rule.validate_file(&make_path("hosts/rewind/default.nix"), content);
        assert!(report.is_some(), "options.networking in hosts/ should trigger");
        assert!(report.unwrap().message.contains("networking"));
    }

    #[test]
    fn test_option_in_nixos_no_report() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          options.nixos.cpuType = lib.mkOption { type = lib.types.enum [ \"amd\" \"intel\" ]; };
          config.nixos.cpuType = \"amd\";
        }";
        let report = rule.validate_file(&make_path("nixos/base.nix"), content);
        assert!(
            report.is_none(),
            "option in non-hosts/ file should not trigger"
        );
    }

    #[test]
    fn test_config_only_in_hosts_no_report() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          config.networking.hostName = \"rewind\";
          config.services.nginx.enable = true;
        }";
        let report = rule.validate_file(&make_path("hosts/rewind/default.nix"), content);
        assert!(
            report.is_none(),
            "config references in hosts/ should not trigger"
        );
    }

    #[test]
    fn test_flake_option_in_hosts_no_report() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          options.flake.description = \"my flake\";
          config.flake.description = \"my flake\";
        }";
        let report = rule.validate_file(&make_path("hosts/rewind/default.nix"), content);
        assert!(
            report.is_none(),
            "options.flake in hosts/ should not trigger (built-in)"
        );
    }

    #[test]
    fn test_deeply_nested_hosts_option() {
        let rule = NoOptionInHosts::new();
        let content = "{ config, lib, ... }: {
          options.session.user = lib.mkOption { type = lib.types.str; };
        }";
        let report = rule.validate_file(&make_path("hosts/rewind/session.nix"), content);
        assert!(
            report.is_some(),
            "option in deeply nested hosts/ file should trigger"
        );
    }
}
