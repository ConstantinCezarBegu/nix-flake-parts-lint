use crate::rnix::{
    SyntaxElement,
    ast::{BinOp, BinOpKind},
};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "bool-equals-true",
    note = "Unnecessary boolean comparison. Use the expression directly.",
    code = 119,
    match_with = NODE_BIN_OP
)]
/// ## What it does
/// Checks for `x == true` and `x == false` comparisons.
///
/// ## Why is this bad?
/// Unnecessary code. Use the boolean expression directly.
pub struct BoolEqualsTrue;

impl Default for BoolEqualsTrue {
    fn default() -> Self {
        Self::new()
    }
}

impl BoolEqualsTrue {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node
            && let Some(bin_op) = BinOp::cast(node.clone())
            && let (Some(lhs), Some(rhs)) = (bin_op.lhs(), bin_op.rhs())
        {
            let lhs_text = lhs.syntax().to_string();
            let rhs_text = rhs.syntax().to_string();
            let is_eq = matches!(bin_op.operator(), Some(BinOpKind::Equal));
            let is_neq = matches!(bin_op.operator(), Some(BinOpKind::NotEqual));

            if is_eq || is_neq {
                let (target, other) = if lhs_text == "true" {
                    (Some("true"), rhs_text)
                } else if lhs_text == "false" {
                    (Some("false"), rhs_text)
                } else if rhs_text == "true" {
                    (Some("true"), lhs_text)
                } else if rhs_text == "false" {
                    (Some("false"), lhs_text)
                } else {
                    (None, String::new())
                };

                if let Some(target) = target
                    && !other.is_empty()
                {
                    let msg = if is_eq {
                        format!("{other} == {target} found. Use {other} directly.")
                    } else {
                        format!("{other} != {target} found. Use !{other} instead.")
                    };
                    return Some(self.report().diagnostic(node.text_range(), msg));
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
        registry.register(Box::new(BoolEqualsTrue::new()));
        registry
    }

    #[test]
    fn test_equals_true_triggers() {
        let src = r#"x == true"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 119);
    }

    #[test]
    fn test_true_equals_triggers() {
        let src = r#"true == x"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 119);
    }

    #[test]
    fn test_equals_false_triggers() {
        let src = r#"x == false"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 119);
    }

    #[test]
    fn test_not_equals_true_triggers() {
        let src = r#"x != true"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 119);
    }

    #[test]
    fn test_not_equals_false_triggers() {
        let src = r#"x != false"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 119);
    }

    #[test]
    fn test_normal_comparison_no_trigger() {
        let src = r#"x == 42"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_normal_boolean_no_trigger() {
        let src = r#"x && y"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
