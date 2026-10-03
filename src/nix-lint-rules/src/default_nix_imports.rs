use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct DefaultNixImports;

impl DefaultNixImports {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultNixImports {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for DefaultNixImports {
    fn code(&self) -> u32 {
        127
    }
    fn name(&self) -> &'static str {
        "default-nix-imports"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "default.nix should only contain imports arrays and flake.modules.*-config wrappers, not inline config/options assignments."
    }

    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport> {
        let path_str = path.to_string_lossy();
        if !path_str.ends_with("default.nix") {
            return None;
        }

        // Find top-level config.X or options.X patterns (first segment only)
        // We flag any where X is NOT "flake" (e.g. config.user, config.networking)
        let config_re = Regex::new(r"(?m)^\s*(config|options)\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=").unwrap();

        for cap in config_re.captures_iter(content) {
            let qualifier = cap.get(1)?.as_str();
            let full_path = cap.get(2)?.as_str();
            let key = full_path.split('.').next()?;

            // Allow config.flake.modules.*-config wrappers
            if key == "flake" {
                continue;
            }

            return Some(FileLevelReport {
                file: path_str.into_owned(),
                message: format!("Top-level {}.{} found in default.nix", qualifier, key),
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
    fn test_darwin_default_nix_valid() {
        let rule = DefaultNixImports::new();
        let content = r#"{ self, inputs, lib, ... }:
{
  imports = [
    ./aerospace
    ./fish.nix
    ./macos-configuration.nix
    ./nix-config.nix
    ./stylix.nix
    ./users.nix
    inputs.home-manager.darwinModules.home-manager
  ];

  options.nix.server.vpnHost = lib.mkOption {
    type = lib.types.str;
    description = "VPN hostname for the server";
  };

  config = {
    flake.modules.darwin.home-manager = inputs.home-manager.darwinModules.home-manager;

    flake.modules.darwin.base =
      { lib, ... }:
      {
        options = {
          secretspecManifest = lib.mkOption { type = lib.types.path; };
        };

        secretspecManifest = builtins.path { name = "spec.toml"; path = ../hosts/secrets/spec.toml; };

        imports = [
          self.modules.darwin.aerospace
          self.modules.darwin.fish
        ];
      };
  };
}"#;
        let report = rule.validate_file(&make_path("darwin/default.nix"), content);
        assert!(
            report.is_some(),
            "Should flag top-level options.nix.server.vpnHost"
        );
    }

    #[test]
    fn test_default_nix_only_imports_and_wrappers() {
        let rule = DefaultNixImports::new();
        let content = r#"{ self, config, lib, inputs, ... }:
{
  imports = [
    ./aerospace
    ./fish.nix
  ];

  flake.modules.darwin.megatron-config = {
    imports = [
      ./networking-option
      ./aerospace-option
    ];
  };
}"#;
        let report = rule.validate_file(&make_path("darwin/megatron/default.nix"), content);
        assert!(
            report.is_none(),
            "Should be valid (only imports and flake.modules wrapper)"
        );
    }

    #[test]
    fn test_default_nix_invalid_inline_config() {
        let rule = DefaultNixImports::new();
        let content = r#"{ self, config, lib, ... }:
{
  config.user.hostname = "test";
  config.networking.hostName = "test";
  flake.modules.nixos.test-config = { imports = []; };
}"#;
        let report = rule.validate_file(&make_path("hosts/bumblebee/default.nix"), content);
        assert!(
            report.is_some(),
            "Should flag top-level config.user and config.networking"
        );
        let msg = report.unwrap().message;
        assert!(
            msg.contains("user") && msg.contains("config"),
            "Message should mention the offending assignment: {}",
            msg
        );
    }

    #[test]
    fn test_non_default_nix_valid() {
        let rule = DefaultNixImports::new();
        let content = r#"{ config, ... }:
{
  config.user.hostname = "test";
  config.networking.hostName = "test";
}"#;
        let report = rule.validate_file(&make_path("hosts/bumblebee/networking-option.nix"), content);
        assert!(
            report.is_none(),
            "non-default.nix should not be checked by this rule"
        );
    }
}
