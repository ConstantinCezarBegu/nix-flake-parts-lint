use crate::rnix::SyntaxElement;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-lookup-path",
    note = "Lookup path found. Use flake inputs instead.",
    code = 106,
    match_with = NODE_PATH
)]
/// ## What it does
/// Checks for lookup paths like `<nixpkgs>`, `<nixos>`, etc.
///
/// ## Why is this bad?
/// Lookup paths use the Nix search path, which is non-reproducible.
pub struct NoLookupPath;

impl Default for NoLookupPath {
    fn default() -> Self {
        Self::new()
    }
}

impl NoLookupPath {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        let text = match node {
            SyntaxElement::Node(n) => n.to_string(),
            SyntaxElement::Token(t) => t.text().to_string(),
        };
        if text.contains("<nixpkgs>")
            || text.contains("<nixos>")
            || text.contains("<nixos/")
            || text.contains("<home-manager>")
            || text.contains("<home-manager/")
            || text.contains("<nix-darwin>")
            || text.contains("<nix-darwin/")
            || text.contains("<nix-channel>")
            || text.contains("<nix-channel/")
            || text.contains("<nix-path>")
            || text.contains("<nix-path/")
        {
            return Some(self.report().diagnostic(
                node.text_range(),
                "Lookup path found. Use flake inputs instead.",
            ));
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
        registry.register(Box::new(NoLookupPath::new()));
        registry
    }

    #[test]
    fn test_nixpkgs_triggers() {
        let src = r#"<nixpkgs>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_nixos_triggers() {
        let src = r#"<nixos>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_home_manager_triggers() {
        let src = r#"<home-manager>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_nix_darwin_triggers() {
        let src = r#"<nix-darwin>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_nix_channel_triggers() {
        let src = r#"<nix-channel>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_nix_path_triggers() {
        let src = r#"<nix-path>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_normal_string_no_trigger() {
        let src = r#"/nix/store/foo"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_nixos_nixpkgs_triggers() {
        let src = r#"<nixos/nixpkgs>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_home_manager_default_triggers() {
        let src = r#"<home-manager/default.nix>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_nixos_modules_triggers() {
        let src = r#"<nixos/modules/services/web-apps/caddy.nix>"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 106);
    }

    #[test]
    fn test_import_no_trigger() {
        let src = r#"import ./foo.nix"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
