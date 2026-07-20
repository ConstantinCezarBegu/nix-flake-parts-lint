use crate::rnix::{SyntaxElement, ast::Ident};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-optional",
    note = "lib.optional/optionally/optionalString found. Use if/then or mkIf.",
    code = 108,
    match_with = NODE_IDENT
)]
/// ## What it does
/// Checks for `lib.optional`, `lib.optionally`, `lib.optionalString` usage.
///
/// ## Why is this bad?
/// These functions hide conditional logic. `lib.optionalAttrs` is excluded as it is idiomatic in NixOS modules.
pub struct NoOptional;

impl Default for NoOptional {
    fn default() -> Self {
        Self::new()
    }
}

impl NoOptional {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(ident) = Ident::cast(node.clone()) {
                let name = ident.to_string();
                if name == "optional" || name == "optionally" || name == "optionalString" {
                    if let Some(parent) = node.parent() {
                        if let Some(grandparent) = parent.parent() {
                            if let Some(select) =
                                crate::rnix::ast::Select::cast(grandparent.clone())
                            {
                                if let Some(expr) = select.expr() {
                                    let expr_text = expr.syntax().to_string();
                                    if expr_text == "lib" || expr_text == "lib.types" {
                                        return Some(self.report().diagnostic(
                                            node.text_range(),
                                            format!(
                                                "lib.{name} found. Use if/then or mkIf instead."
                                            ),
                                        ));
                                    }
                                }
                            }
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
    #![allow(dead_code)]
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoOptional::new()));
        registry
    }

    #[test]
    fn test_lib_optional_triggers() {
        let src = r#"lib.optional true x"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 108);
    }

    #[test]
    fn test_lib_optionally_triggers() {
        let src = r#"lib.optionally true x"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 108);
    }

    #[test]
    fn test_lib_optional_string_triggers() {
        let src = r#"lib.optionalString true x"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 108);
    }

    #[test]
    fn test_standalone_optional_no_trigger() {
        let src = r#"optional true x"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
