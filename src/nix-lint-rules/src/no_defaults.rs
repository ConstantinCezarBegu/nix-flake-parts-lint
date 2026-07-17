use nix_lint_core::{Metadata, Report};
use rowan::ast::AstNode;
use rnix::{SyntaxElement, ast::{AttrSet, HasEntry}};

#[nix_lint_macros::lint(
    name = "no-defaults",
    note = "mkOption with default found. All custom options must be set explicitly per host.",
    code = 113,
    match_with = NODE_ATTR_SET
)]
/// ## What it does
/// Checks for mkOption blocks with `default = ` fields.
///
/// ## Why is this bad?
/// All custom options should be set explicitly per host, not have defaults.
pub struct NoDefaults;

impl NoDefaults {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(attrset) = AttrSet::cast(node.clone()) {
                let parent_text = node.parent().map(|p| p.to_string()).unwrap_or_default();
                if !parent_text.contains("mkOption") {
                    return None;
                }
                for entry in attrset.attrpath_values() {
                    if let Some(attrpath) = entry.attrpath() {
                        let mut attrs: Vec<_> = attrpath.attrs().collect();
                        if attrs.len() == 1 {
                            if let Some(ident) = rnix::ast::Ident::cast(attrs.pop().unwrap().syntax().clone()) {
                                if ident.to_string() == "default" {
                                    return Some(self.report().diagnostic(node.text_range(), "mkOption with default found. All custom options must be set explicitly per host."));
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
        registry.register(Box::new(NoDefaults::new()));
        registry
    }

    #[test]
    fn test_mkoption_with_default_triggers() {
        let src = r#"{ options.foo = lib.mkOption {
          type = lib.types.bool;
          default = false;
        }; }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 113);
    }

    #[test]
    fn test_attrset_no_default_no_trigger() {
        let src = r#"{
          foo = "bar";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nested_sources_default_no_trigger() {
        let src = r#"{
          sources.default = [ pkgs.nixvim ];
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_mkoption_without_default_no_trigger() {
        let src = r#"{
          type = lib.types.str;
          description = "A foo option";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
