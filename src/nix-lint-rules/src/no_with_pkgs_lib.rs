use crate::rnix::{SyntaxElement, ast::With};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-with-pkgs-lib",
    note = "with pkgs/lib found. Import specific identifiers explicitly.",
    code = 101,
    match_with = NODE_WITH
)]
/// ## What it does
/// Checks for `with pkgs;` or `with lib;` patterns.
///
/// ## Why is this bad?
/// `with` expressions shadow the namespace.
pub struct NoWithPkgsLib;

impl Default for NoWithPkgsLib {
    fn default() -> Self {
        Self::new()
    }
}

impl NoWithPkgsLib {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(with_expr) = With::cast(node.clone()) {
                if let Some(namespace) = with_expr.namespace() {
                    let text = namespace.syntax().to_string();
                    if text == "pkgs" || text == "lib" {
                        return Some(self.report().diagnostic(
                            node.text_range(),
                            format!("with {text} found. Import specific identifiers explicitly."),
                        ));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoWithPkgsLib::new()));
        registry
    }

    #[test]
    fn test_with_pkgs_triggers() {
        let src = r#"with pkgs; {
          foo = bar;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 101);
    }

    #[test]
    fn test_with_lib_triggers() {
        let src = r#"with lib; {
          foo = bar;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 101);
    }

    #[test]
    fn test_with_custom_no_trigger() {
        let src = r#"with myPkgs; {
          foo = bar;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_no_with_no_trigger() {
        let src = r#"{
          foo = import ./foo.nix;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_with_pkgs_in_let_triggers() {
        let src = r#"{ foo = let x = with pkgs; hello; in x; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 101);
    }

    #[test]
    fn test_with_lib_in_let_triggers() {
        let src = r#"{ lib, ... }: let
          x = with lib; concatStringsSep "," ["a" "b"];
        in { }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 101);
    }

    #[test]
    fn test_with_nixpkgs_no_trigger() {
        let src = r#"with nixpkgs; {
          foo = bar;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_with_nixpkgs_lib_no_trigger() {
        let src = r#"with nixpkgs.lib; {
          foo = bar;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
