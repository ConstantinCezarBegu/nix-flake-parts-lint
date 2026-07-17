use nix_lint_core::{Metadata, Report};
use rowan::ast::AstNode;
use rnix::{SyntaxElement, ast::{Attrpath, Ident, Select}};

#[nix_lint_macros::lint(
    name = "no-any-type",
    note = "types.anything found. Use a specific type for proper validation.",
    code = 105,
    match_with = NODE_IDENT
)]
/// ## What it does
/// Checks for `types.anything` usage.
///
/// ## Why is this bad?
/// `lib.types.anything` provides no type safety.
pub struct NoAnyType;

impl NoAnyType {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(ident) = Ident::cast(node.clone()) {
                if ident.to_string() == "anything" {
                    if let Some(parent) = node.parent() {
                        if let Some(attrpath) = Attrpath::cast(parent.clone()) {
                            let attrs: Vec<_> = attrpath.attrs().into_iter().collect();
                            if let Some(last) = attrs.last() {
                                if last.to_string() == "anything" {
                                    if let Some(grandparent) = parent.parent() {
                                        if let Some(select) = Select::cast(grandparent.clone()) {
                                            for attr in &attrs {
                                                if attr.to_string() == "types" {
                                                    return Some(self.report().diagnostic(node.text_range(), "types.anything found. Use a specific type for proper validation."));
                                                }
                                            }
                                            if let Some(expr) = select.expr() {
                                                if expr.syntax().to_string() == "types" {
                                                    return Some(self.report().diagnostic(node.text_range(), "types.anything found. Use a specific type for proper validation."));
                                                }
                                            }
                                        }
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
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoAnyType::new()));
        registry
    }

    #[test]
    fn test_types_anything_triggers() {
        let src = r#"lib.types.anything"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 105);
    }

    #[test]
    fn test_types_anything_no_lib_prefix_triggers() {
        let src = r#"types.anything"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 105);
    }

    #[test]
    fn test_other_type_no_trigger() {
        let src = r#"lib.types.string"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_types_attrs_no_trigger() {
        let src = r#"lib.types.attrs"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_standalone_anything_no_trigger() {
        let src = r#"anything"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_types_anything_in_either_triggers() {
        let src = r#"lib.types.either lib.types.anything (lib.types.str)"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 105);
    }
}
