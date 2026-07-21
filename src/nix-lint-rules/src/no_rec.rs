use crate::rnix::{SyntaxElement, ast::AttrSet};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-rec",
    note = "rec {} usage found. Use explicit let bindings instead.",
    code = 100,
    match_with = NODE_ATTR_SET
)]
/// ## What it does
/// Checks for `rec {}` usage.
///
/// ## Why is this bad?
/// `rec` blocks encourage infinite recursion and make the dependency
/// graph implicit. Use explicit `let` bindings instead.
pub struct NoRec;

impl Default for NoRec {
    fn default() -> Self {
        Self::new()
    }
}

impl NoRec {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(syntax) = node
            && let Some(attrset) = AttrSet::cast(syntax.clone())
            && attrset.rec_token().is_some()
        {
            return Some(
                self.report()
                    .diagnostic(node.text_range(), "rec {} found. Use let bindings instead."),
            );
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
        registry.register(Box::new(NoRec::new()));
        registry
    }

    #[test]
    fn test_no_rec_rec_attrset_triggers() {
        let src = r#"rec {
          foo = bar;
          bar = 42;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 100);
    }

    #[test]
    fn test_no_rec_normal_attrset_no_trigger() {
        let src = r#"{
          foo = bar;
          bar = 42;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_no_rec_rec_function_no_trigger() {
        let src = r#"{
          fn = rec {
            f = x: x;
          };
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 100);
    }

    #[test]
    fn test_no_rec_rec_nested_no_trigger() {
        let src = r#"{
          foo = rec {
            bar = 42;
          };
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 100);
    }

    #[test]
    fn test_no_rec_rec_with_let_no_trigger() {
        let src = r#"{
          foo = let x = 42; in x;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
