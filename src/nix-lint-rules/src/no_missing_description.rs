use crate::rnix::{SyntaxElement, ast::AttrSet};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-missing-description",
    note = "mkOption without description found. All options must be documented.",
    code = 111,
    match_with = NODE_ATTR_SET
)]
/// ## What it does
/// Checks for mkOption blocks that don't have a description field.
///
/// ## Why is this bad?
/// Options without descriptions make the module harder to understand.
pub struct NoMissingDescription;

impl Default for NoMissingDescription {
    fn default() -> Self {
        Self::new()
    }
}

impl NoMissingDescription {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(_attrset) = AttrSet::cast(node.clone()) {
                let text = node.to_string();
                if text.contains("mkOption") && !text.contains("description") {
                    return Some(
                        self.report()
                            .diagnostic(node.text_range(), "mkOption without description found."),
                    );
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
        registry.register(Box::new(NoMissingDescription::new()));
        registry
    }

    #[test]
    fn test_mkoption_without_description_triggers() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.bool;
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 111);
    }

    #[test]
    fn test_mkoption_with_description_no_trigger() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.bool;
          description = "A foo option";
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_attrset_without_mkoption_no_trigger() {
        let src = r#"{
          foo = "bar";
          baz = 42;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_mkoption_with_default_no_description_triggers() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.str;
          default = "hello";
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 111);
    }

    #[test]
    fn test_mkoption_with_example_no_description_triggers() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.str;
          example = "hello";
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 111);
    }

    #[test]
    fn test_mkoption_with_apply_no_description_triggers() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.bool;
          apply = v: v;
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 111);
    }
}
