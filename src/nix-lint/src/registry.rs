//! Registry of all built-in lint rules.

use nix_lint_core::LintRegistry;

pub fn build_registry() -> LintRegistry {
    let mut registry = LintRegistry::new();

    // Node-level rules (AST-based)
    registry.register(Box::new(nix_lint_rules::NoRec::new()));
    registry.register(Box::new(nix_lint_rules::NoWithPkgsLib::new()));
    registry.register(Box::new(nix_lint_rules::NoMkDefault::new()));
    registry.register(Box::new(nix_lint_rules::NoMkForce::new()));
    registry.register(Box::new(nix_lint_rules::NoMkIfTrue::new()));
    registry.register(Box::new(nix_lint_rules::NoAnyType::new()));
    registry.register(Box::new(nix_lint_rules::NoLookupPath::new()));
    registry.register(Box::new(nix_lint_rules::NoNixEnv::new()));
    registry.register(Box::new(nix_lint_rules::NoOptional::new()));
    registry.register(Box::new(nix_lint_rules::NoFirewallDisable::new()));
    registry.register(Box::new(nix_lint_rules::NoBuiltinReadfileSecrets::new()));
    registry.register(Box::new(nix_lint_rules::NoMissingDescription::new()));
    registry.register(Box::new(nix_lint_rules::NoSecrets::new()));
    registry.register(Box::new(nix_lint_rules::NoDefaults::new()));
    registry.register(Box::new(nix_lint_rules::BoolEqualsTrue::new()));

    // File-level rules (text analysis on full files)
    registry.register_file_level(Box::new(nix_lint_rules::OneProgramPerPart::new()));
    registry.register_file_level(Box::new(nix_lint_rules::RequireFlakeParts::new()));
    registry.register_file_level(Box::new(nix_lint_rules::RequireAssertions::new()));
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossNamespaceWrites::new()));
    registry.register_file_level(Box::new(nix_lint_rules::NoCrossModuleOptionReads::new()));

    registry
}
