use crate::rnix::{SyntaxElement, ast::Apply};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-mkif-true",
    note = "mkIf condition true found. Use the condition directly.",
    code = 104,
    match_with = NODE_IDENT
)]
/// ## What it does
/// Checks for `mkIf true ...` or `mkIf condition true` patterns.
///
/// ## Why is this bad?
/// `mkIf condition true` is equivalent to just `condition`.
pub struct NoMkIfTrue;

impl Default for NoMkIfTrue {
    fn default() -> Self {
        Self::new()
    }
}

impl NoMkIfTrue {
    fn is_mkif_lambda(&self, lambda_text: &str) -> bool {
        lambda_text == "lib.mkIf" || lambda_text == "mkIf"
    }

    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node
            && let Some(ident) = crate::rnix::ast::Ident::cast(node.clone())
            && ident.to_string() != "true"
        {
            return None;
        }
        if let SyntaxElement::Node(node) = node
            && let Some(ident) = crate::rnix::ast::Ident::cast(node.clone())
            && ident.to_string() == "true"
            && let Some(parent) = node.parent()
            && let Some(parent_apply) = Apply::cast(parent.clone())
        {
            if let Some(lambda) = parent_apply.lambda() {
                let func_text = lambda.syntax().to_string();
                if self.is_mkif_lambda(&func_text) {
                    return Some(self.report().diagnostic(
                        node.text_range(),
                        "mkIf condition true found. Use the condition directly.",
                    ));
                }
            }
            if let Some(arg) = parent_apply.argument()
                && arg.syntax().text_range() == node.text_range()
                && let Some(lambda2) = parent_apply.lambda()
            {
                let func_text = lambda2.syntax().to_string();
                if func_text.contains("mkIf") {
                    return Some(self.report().diagnostic(
                        node.text_range(),
                        "mkIf condition true found. Use the condition directly.",
                    ));
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
        registry.register(Box::new(NoMkIfTrue::new()));
        registry
    }

    #[test]
    fn test_mkif_true_triggers() {
        let src = r#"mkIf true (throw "hi")"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 104);
    }

    #[test]
    fn test_lib_mkif_true_triggers() {
        let src = r#"lib.mkIf true (throw "hi")"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 104);
    }

    #[test]
    fn test_mkif_condition_true_triggers() {
        let src = r#"mkIf someCondition true"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 104);
    }

    #[test]
    fn test_mkif_paren_condition_true_triggers() {
        let src = r#"lib.mkIf (config.hardware.gpuType == "amd") true"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 104);
    }

    #[test]
    fn test_plain_true_no_trigger() {
        let src = r#"true"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_mkif_false_no_trigger() {
        let src = r#"mkIf false (throw "hi")"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_mkif_true_attrset_triggers() {
        let src = r#"mkIf true { enable = true; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 104);
    }

    #[test]
    fn test_true_inside_mkif_body_no_trigger() {
        let src = r#"lib.mkIf (x == 1) { enable = true; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_true_in_nested_assertion_no_trigger() {
        let src = r#"lib.mkIf config.nixos.hasHibernate {
          assertions = [{ assertion = true; message = "ok"; }];
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
