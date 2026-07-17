use nix_lint_core::{Metadata, Report};
use rowan::ast::AstNode;
use rnix::{SyntaxElement, ast::Str};

#[nix_lint_macros::lint(
    name = "no-nix-env",
    note = "Imperative nix-env usage found. Use declarative package management.",
    code = 107,
    match_with = NODE_STRING
)]
/// ## What it does
/// Checks for imperative `nix-env` usage.
///
/// ## Why is this bad?
/// `nix-env` is imperative and not reproducible.
pub struct NoNixEnv;

impl NoNixEnv {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(_s) = Str::cast(node.clone()) {
                let text = node.to_string();
                if text.contains("nix-env ") || text.contains("nix-env\t") {
                    for flag in &["nix-env -i", "nix-env -e", "nix-env -l", "nix-env -p", "nix-env -r", "nix-env -U", "nix-env -q", "nix-env -I"] {
                        if text.contains(flag) {
                            return Some(self.report().diagnostic(node.text_range(), "Imperative nix-env usage found. Use declarative package management."));
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoNixEnv::new()));
        registry
    }

    #[test]
    fn test_nix_env_install_triggers() {
        let src = r#""nix-env -i nixpkgs.hello""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 107);
    }

    #[test]
    fn test_nix_env_uninstall_triggers() {
        let src = r#""nix-env -e mypackage""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 107);
    }

    #[test]
    fn test_nix_env_query_triggers() {
        let src = r#""nix-env -q""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 107);
    }

    #[test]
    fn test_nix_env_upgrade_triggers() {
        let src = r#""nix-env -U https://example.com/nix-cache""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 107);
    }

    #[test]
    fn test_normal_string_no_trigger() {
        let src = r#""just a string""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nix_shell_no_trigger() {
        let src = r#""nix-shell -p hello""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nix_run_no_trigger() {
        let src = r#""nix run nixpkgs#hello""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nix_env_in_comment_no_trigger() {
        let src = r#"{
          # This is not nix-env -i
          foo = "bar";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nix_env_build_expr_no_trigger() {
        let src = r#""/nix/store/xxx-nix-env-2.18.0""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
