use crate::rnix::{SyntaxElement, ast::Ident};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-mkforce",
    note = "mkForce usage found. Use module priority instead.",
    code = 103,
    match_with = NODE_IDENT
)]
/// ## What it does
/// Checks for `mkForce` usage.
///
/// ## Why is this bad?
/// `mkForce` breaks the module system's priority model.
pub struct NoMkForce;

impl Default for NoMkForce {
    fn default() -> Self {
        Self::new()
    }
}

impl NoMkForce {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node
            && let Some(ident) = Ident::cast(node.clone())
            && ident.to_string() == "mkForce"
        {
            return Some(self.report().diagnostic(
                node.text_range(),
                "mkForce found. Use module priority or proper composition instead.",
            ));
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
        registry.register(Box::new(NoMkForce::new()));
        registry
    }

    #[test]
    fn test_mkforce_triggers() {
        let src = r#"mkForce 42"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 103);
    }

    #[test]
    fn test_lib_mkforce_triggers() {
        let src = r#"lib.mkForce "value""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 103);
    }

    #[test]
    fn test_other_ident_no_trigger() {
        let src = r#"myForce 42"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
