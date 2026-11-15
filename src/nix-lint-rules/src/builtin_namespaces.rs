//! Namespaces shared by the cross-namespace rules (117 writes, 118 reads):
//! option trees declared by flake-parts, nixpkgs, nix-darwin and
//! home-manager themselves, which any module may use without declaring them.

pub(crate) static BUILTIN_OPTIONS: &[&str] = &[
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
    // The module system's own `lib` option (NixOS and home-manager), where
    // modules publish helpers: config.lib.stylix.colors, config.lib.file.
    "lib",
];

/// Whether the `config.` starting at byte `start` is part of a file name or
/// other dotted word rather than a reference to the config tree, e.g. the
/// "config.yaml" in `xdg.configFile."my-app/config.yaml"` or `"config.json"`.
pub(crate) fn in_file_name(content: &str, start: usize) -> bool {
    // A real reference never directly follows a quote: inside a string it
    // is always interpolated, preceded by `${`.
    matches!(
        content[..start].chars().next_back(),
        Some('/' | '.' | '-' | '"')
    )
}
