use std::sync::LazyLock;

use crate::rnix::{
    NixLanguage, SyntaxElement,
    ast::{AttrpathValue, Str},
};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};
use regex::Regex;

static SECRET_KEYWORD_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"^api[_\-]?key$").unwrap(),
        Regex::new(r"^api[_\-]?token$").unwrap(),
        Regex::new(r"^password$").unwrap(),
        Regex::new(r"^secret[_\-]?\w*$").unwrap(),
        Regex::new(r"^\w*[_\-]?token$").unwrap(),
    ]
});

#[nix_lint_macros::lint(
    name = "no-secrets",
    note = "Potential secrets in source found. Use agenix or sops-nix.",
    code = 112,
    match_with = NODE_STRING
)]
/// ## What it does
/// Checks for plaintext secrets in Nix source files.
///
/// ## Why is this bad?
/// Secrets should be encrypted using agenix or sops-nix.
pub struct NoSecrets;

impl Default for NoSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl NoSecrets {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node
            && let Some(_str) = Str::cast(node.clone())
        {
            let text = node.to_string();

            // Check for private key content
            if text.contains("BEGIN") && text.contains("PRIVATE KEY") {
                return Some(self.report().diagnostic(
                    node.text_range(),
                    "Potential secrets in source found. Use agenix or sops-nix.",
                ));
            }

            // Check if string is a value of an attribute with a secret-like key name
            if self.is_secret_value(node) {
                return Some(self.report().diagnostic(
                    node.text_range(),
                    "Potential secrets in source found. Use agenix or sops-nix.",
                ));
            }
        }
        None
    }

    fn is_secret_value(&self, str_node: &crate::rowan::SyntaxNode<NixLanguage>) -> bool {
        let text = str_node.to_string();

        // Skip empty strings and very short strings
        if text.len() <= 2 {
            return false;
        }

        // Skip placeholder patterns like @SEARX_SECRET_KEY@ (with or without quotes)
        let inner = text.trim_matches('"');
        if inner.starts_with('@') && inner.ends_with('@') {
            return false;
        }

        // Check if this string is a value in an AttrpathValue with a secret-like key
        if let Some(parent) = str_node.parent()
            && let Some(attrpath_value) = AttrpathValue::cast(parent.clone())
            && let Some(attrpath) = attrpath_value.attrpath()
        {
            for attr in attrpath.attrs() {
                if let Some(ident) = crate::rnix::ast::Ident::cast(attr.syntax().clone()) {
                    let key_text = ident.to_string().to_lowercase();
                    if self.matches_secret_keyword(&key_text) && !self.is_prose(&text) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn matches_secret_keyword(&self, key: &str) -> bool {
        for pattern in &*SECRET_KEYWORD_PATTERNS {
            if pattern.is_match(key) {
                return true;
            }
        }
        false
    }

    fn is_prose(&self, text: &str) -> bool {
        let words: Vec<_> = text.split_whitespace().collect();
        words.len() >= 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix_lint_core::LintRegistry;

    fn make_registry() -> LintRegistry {
        let mut registry = LintRegistry::new();
        registry.register(Box::new(NoSecrets::new()));
        registry
    }

    #[test]
    fn test_password_key_triggers() {
        let src = r#"{
          password = "supersecret123";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_api_key_triggers() {
        let src = r#"{
          api_key = "abc123";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_secret_key_triggers() {
        let src = r#"{
          secret_key = "abc123";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_token_triggers() {
        let src = r#"{
          auth_token = "abc123";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_placeholder_no_trigger() {
        let src = r#"{
          password = "@SEARX_SECRET_KEY@";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_prose_no_trigger() {
        let src = r#"{
          password = "This is a normal sentence with multiple words";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_api_key_hyphen_triggers() {
        let src = r#"{
          api-key = "abc123";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_nested_service_password_triggers() {
        let src = r#"{
          services.myApp.password = "hunter2";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_empty_string_no_trigger() {
        let src = r#"{
          password = "";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }

    #[test]
    fn test_private_key_content_triggers() {
        let src = r#"{
          privateKey = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBg...
-----END PRIVATE KEY-----";
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].code, 112);
    }

    #[test]
    fn test_agenix_reference_no_trigger() {
        let src = r#"{
          password = config.age.secrets.pass.path;
        }"#;
        let reports = nix_lint_core::lint_file(&make_registry(), src).unwrap();
        assert!(reports.is_empty());
    }
}
