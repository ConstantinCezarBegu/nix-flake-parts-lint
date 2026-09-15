use std::collections::HashSet;
use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

static BUILTIN_OPTIONS: &[&str] = &[
    // flake-parts built-in outputs
    "flake",
    "perSystem",
    // nixpkgs system options
    "system",
    // nixpkgs core options
    "nix",
    "nixpkgs",
    "environment",
    // nixpkgs user/account options
    "users",
    "groups",
    // nixpkgs boot options
    "boot",
    // nixpkgs networking options
    "networking",
    // nixpkgs security options
    "security",
    // nixpkgs hardware options
    "hardware",
    // nixpkgs localization options
    "i18n",
    "locale",
    // nixpkgs time options
    "time",
    // nixpkgs service options
    "services",
    // nixpkgs virtualization options
    "virtualisation",
    "containers",
    // nixpkgs program options
    "programs",
    // nixpkgs documentation options
    "documentation",
    // nixpkgs font options
    "fonts",
    // nixpkgs XDG options
    "xdg",
    // nixpkgs power management options
    "powerManagement",
    // nixpkgs sound options
    "sound",
    // nix-darwin options
    "darwin",
    // home-manager options
    "home-manager",
    // nixos-generators options
    "nixosConfigurations",
    // nixosModules / darwinModules
    "nixosModules",
    "darwinModules",
    "homeManagerModules",
    "homeModules",
];

pub struct NoCrossNamespaceWrites;

impl NoCrossNamespaceWrites {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoCrossNamespaceWrites {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for NoCrossNamespaceWrites {
    fn code(&self) -> u32 {
        117
    }
    fn name(&self) -> &'static str {
        "no-cross-namespace-writes"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Module writes to config namespace not declared as options in this file."
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
        let declared_set: HashSet<&str> = declared.iter().copied().collect();
        let builtin_set: HashSet<&str> = BUILTIN_OPTIONS.iter().copied().collect();

        // Match all config.X writes where X is a namespace.
        // Uses \b to match start-of-line and word boundaries.
        // Checks that the char after the namespace is not an identifier continuation char
        // (or we're at end of string) to avoid matching partial identifiers.
        let config_write_re = Regex::new(r"\bconfig\.([a-zA-Z_]\w*)").unwrap();
        for cap in config_write_re.captures_iter(content) {
            let ns = cap.get(1)?.as_str();
            let match_end = cap.get(1).unwrap().end();
            let next_char = content[match_end..].chars().next();
            // Skip if the namespace continues (next char is a valid identifier char)
            if next_char.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            if !declared_set.contains(ns) && !builtin_set.contains(ns) {
                return Some(FileLevelReport {
                    file: path.to_string_lossy().into_owned(),
                    message: format!(
                        "Module writes to config.{} but does not declare options.{} in this file.",
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

    #[test]
    fn test_builtin_nix_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.nix.settings.auto-optimise-store = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_services_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.enable = lib.mkOption { type = lib.types.bool; };
          config.services.nginx.enable = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_environment_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.package = lib.mkOption { type = lib.types.package; };
          config.environment.systemPackages = [ config.myService.package ];
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_flake_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.flake.description = "my flake";
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_darwin_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.darwin.enableAutologin = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_programs_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.programs.git.enable = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_user_option_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.users.users.myuser.isNormalUser = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_multiple_builtin_options_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.enable = lib.mkOption { type = lib.types.bool; };
          config.nix.settings.auto-optimise-store = true;
          config.services.nginx.enable = true;
          config.environment.systemPackages = [ ];
          config.darwin.enableAutologin = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_builtin_and_declared_mixed_no_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.nix.settings.auto-optimise-store;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_non_builtin_undeclared_namespace_report() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.undeclaredThing.bar = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_some());
        let report = report.unwrap();
        assert!(report.message.contains("undeclaredThing"));
    }

    #[test]
    fn test_config_write_at_start_of_line() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
config.services.nginx.enable = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_assignment_no_trailing_dot() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.services = { nginx.enable = true; };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_config_comparison_no_trailing_dot() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          config.myService.bar = config.services == { };
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }

    #[test]
    fn test_myconfig_write_not_matched() {
        let rule = NoCrossNamespaceWrites::new();
        let content = r#"{ config, lib, ... }: {
          options.myService.foo = lib.mkOption { type = lib.types.bool; };
          myConfig.services.nginx.enable = true;
        }"#;
        let report = rule.validate_file(&make_path("test.nix"), content);
        assert!(report.is_none());
    }
}
