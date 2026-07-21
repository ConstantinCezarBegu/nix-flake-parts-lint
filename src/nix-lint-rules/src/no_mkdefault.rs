use crate::rnix::{SyntaxElement, ast::Ident};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-mkdefault",
    note = "mkDefault usage found. Set values explicitly in host config.",
    code = 102,
    match_with = NODE_IDENT
)]
/// ## What it does
/// Checks for `mkDefault` usage.
///
/// ## Why is this bad?
/// `mkDefault` makes it unclear where default values come from.
pub struct NoMkDefault;

impl Default for NoMkDefault {
    fn default() -> Self {
        Self::new()
    }
}

impl NoMkDefault {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node
            && let Some(ident) = Ident::cast(node.clone())
            && ident.to_string() == "mkDefault"
        {
            return Some(self.report().diagnostic(
                node.text_range(),
                "mkDefault found. Set values explicitly in host config.",
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
        registry.register(Box::new(NoMkDefault::new()));
        registry
    }

    #[test]
    fn test_mkdefault_triggers() {
        let src = r#"mkDefault 42"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 102);
    }

    #[test]
    fn test_lib_mkdefault_triggers() {
        let src = r#"lib.mkDefault "value""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 102);
    }

    #[test]
    fn test_mkooverride_no_trigger() {
        let src = r#"lib.mkOverride 500 "value""#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_other_ident_no_trigger() {
        let src = r#"myDefault 42"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
