use nix_lint_core::{Metadata, Report};
use rowan::ast::AstNode;
use rnix::{SyntaxElement, ast::Attrpath};

#[nix_lint_macros::lint(
    name = "no-firewall-disable",
    note = "firewall.enable = false found. This is a security risk.",
    code = 109,
    match_with = NODE_ATTRPATH
)]
/// ## What it does
/// Checks for `firewall.enable = false` patterns.
///
/// ## Why is this bad?
/// Disabling the firewall is a security risk.
pub struct NoFirewallDisable;

impl NoFirewallDisable {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(_attrpath) = Attrpath::cast(node.clone()) {
                let text = node.to_string();
                if text.contains("firewall") && text.contains("enable") {
                    // Check parent chain for false
                    if let Some(parent) = node.parent() {
                        let parent_text = parent.to_string();
                        if parent_text.contains("false") {
                            return Some(self.report().diagnostic(node.text_range(), "firewall.enable = false found. This is a security risk."));
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoFirewallDisable::new()));
        registry
    }

    #[test]
    fn test_firewall_enable_false_triggers() {
        let src = r#"{
          networking.firewall.enable = false;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 109);
    }

    #[test]
    fn test_firewall_enable_true_no_trigger() {
        let src = r#"{
          networking.firewall.enable = true;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
                    }
                }
            }
        }
        None
    }
}
