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
        // `=[^=]` (not just `\s*=`) so this requires a real assignment, not
        // a comparison (==) — which matters when e.g. a long
        // `child.enable = config.foo.bar == "baz";` line gets wrapped by
        // nixfmt, landing `config.foo.bar == "baz";` alone at the start of
        // its own line.
        let config_re = Regex::new(r"(?m)^\s*(config|options)\.([a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)*)\s*=[^=]").unwrap();

        for cap in config_re.captures_iter(content) {
            let qualifier = cap.get(1)?.as_str();
            let full_path = cap.get(2)?.as_str();
            let key = full_path.split('.').next()?;

            // Allow config.flake.modules.*-config wrappers
            if key == "flake" {
                continue;
            }

            // Nixvim keymap entries carry their own `options.silent = true;`
            // field (unrelated to NixOS module options) — not a module
            // option/config assignment at all.
            if key == "silent" {
                let before = &content[..cap.get(0)?.start()];
                if before.contains("keymaps") || before.contains("programs.nixvim") {
                    continue;
                }
            }

            // Allow a top-level option whose value is pushed down into a
            // child module's own, more specific option (mirrors
            // nixos.hardware.cpuType = config.nixos.cpuType; for the
            // cpuType/gpuType pattern): look for a *different*, dotted
            // (multi-segment) assignment on the SAME LINE that reads
            // config.<path>. A plain single-identifier let-binding (no dot)
            // doesn't count as delegation, and the match must not cross
            // lines — otherwise it would also match across the file's own
            // `flake.modules.X.Y = { ... }:` wrapper (always dotted, always
            // present), which isn't delegation, it's just the module header.
            if qualifier == "options" {
                let delegation_re = Regex::new(&format!(
                    r"(?m)^\s*[a-zA-Z_][a-zA-Z0-9_\-]*(?:\.[a-zA-Z_][a-zA-Z0-9_\-]*)+\s*=[^=;\n][^;\n]*\bconfig\.{}\b",
                    regex::escape(full_path)
                ))
                .ok()?;
                if delegation_re.is_match(content) {
                    continue;
                }
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

    #[test]
    fn test_nixvim_keymap_silent_option_valid() {
        let rule = DefaultNixImports::new();
        let content = r#"{ ... }:
{
  imports = [ ./insert-mode.nix ];

  flake.modules.homeManager.keymaps = {
    programs.nixvim.keymaps = [
      {
        mode = "n";
        key = "<leader>w";
        action = "<cmd>w<CR>";
        options.silent = true;
      }
    ];
  };
}"#;
        let report = rule.validate_file(&make_path("development/nixvim/keymaps/default.nix"), content);
        assert!(
            report.is_none(),
            "a nixvim keymap's options.silent field is not a module option/config assignment"
        );
    }

    #[test]
    fn test_option_delegated_to_child_option_valid() {
        let rule = DefaultNixImports::new();
        // The second assignment is wrapped across two lines exactly as
        // nixfmt would format it once the line gets too long — the
        // continuation line starts with `config.<path> == "svalboard";`,
        // which must not be mistaken for a new top-level `config.X =`
        // assignment (the `=` there is the first char of `==`, not a real
        // assignment).
        let content = r#"{ lib, config, ... }:
{
  options.wayland.sway.controls.keybindingLayout = lib.mkOption {
    type = lib.types.enum [ "qwerty" "svalboard" ];
  };

  config = {
    wayland.sway.controls.qwerty.enable = config.wayland.sway.controls.keybindingLayout == "qwerty";
    wayland.sway.controls.svalboard.enable =
      config.wayland.sway.controls.keybindingLayout == "svalboard";
  };
}"#;
        let report = rule.validate_file(&make_path("wayland/sway/controls/default.nix"), content);
        assert!(
            report.is_none(),
            "an option pushed down into a child module's own option (mirroring cpuType/gpuType) should be valid"
        );
    }

    #[test]
    fn test_option_local_let_binding_still_invalid() {
        let rule = DefaultNixImports::new();
        // Mirrors the real file's shape: the option is declared and read
        // *inside* the flake.modules.X.Y = { ... }: wrapper, not at the
        // outer top level. A naive delegation search spanning multiple
        // lines would wrongly treat the dotted wrapper assignment itself
        // (flake.modules.darwin.aerospace = ...) as "delegation" just
        // because config.<path> appears somewhere later before any
        // semicolon — this must stay flagged.
        let content = r#"{ lib, ... }:
{
  flake.modules.darwin.aerospace =
    {
      config,
      pkgs,
      ...
    }:
    let
      keybindingFile =
        if config.darwin.aerospace.keybindingLayout == "svalboard" then ./svalboard.nix else ./qwerty.nix;
    in
    {
      options.darwin.aerospace.keybindingLayout = lib.mkOption {
        type = lib.types.enum [ "qwerty" "svalboard" ];
      };

      config = {
        services.aerospace.enable = true;
      };
    };
}"#;
        let report = rule.validate_file(&make_path("darwin/aerospace/default.nix"), content);
        assert!(
            report.is_some(),
            "reading the option into a plain (non-dotted) let-binding is local consumption, not delegation to a child option, and should still be flagged even though the file's own flake.modules.X.Y = wrapper is a dotted assignment too"
        );
    }
}
