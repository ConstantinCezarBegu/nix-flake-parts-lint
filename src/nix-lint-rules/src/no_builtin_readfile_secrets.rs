use crate::rnix::{SyntaxElement, ast::Apply};
use crate::rowan::ast::AstNode;
use nix_lint_core::{Metadata, Report};

#[nix_lint_macros::lint(
    name = "no-builtin-readfile-secrets",
    note = "builtins.readFile for potential secrets found. Use agenix/sops-nix.",
    code = 110,
    match_with = NODE_APPLY
)]
/// ## What it does
/// Checks for `builtins.readFile` used with secret file patterns.
///
/// ## Why is this bad?
/// Reading secret files with `builtins.readFile` bypasses encryption.
pub struct NoBuiltinReadfileSecrets;

impl Default for NoBuiltinReadfileSecrets {
    fn default() -> Self {
        Self::new()
    }
}

impl NoBuiltinReadfileSecrets {
    fn check(&self, node: &SyntaxElement) -> Option<Report> {
        if let SyntaxElement::Node(node) = node {
            if let Some(_apply) = Apply::cast(node.clone()) {
                let text = node.to_string();
                if text.contains("builtins.readFile") {
                    let patterns = [
                        ".age", ".pub", ".pem", ".crt", ".key", ".env", ".secret", "password",
                        "token", "secret",
                    ];
                    for pattern in &patterns {
                        if text.contains(pattern) {
                            return Some(self.report().diagnostic(node.text_range(), "builtins.readFile for potential secrets found. Use agenix/sops-nix instead."));
                        }

                        #[cfg(test)]
                        mod tests {
                            #![allow(dead_code)]
                            use super::*;
                            use nix_lint_core::LintRegistry;

                            fn make_registry() -> LintRegistry {
                                let mut registry = LintRegistry::new();
                                registry.register(Box::new(NoBuiltinReadfileSecrets::new()));
                                registry
                            }

                            #[test]
                            fn test_readfile_age_triggers() {
                                let src = r#"builtins.readFile ./secret.age"#;
                                let reports =
                                    nix_lint_core::lint_file(&make_registry(), src).unwrap();
                                assert!(!reports.is_empty());
                                assert_eq!(reports[0].code, 110);
                            }

                            #[test]
                            fn test_readfile_key_triggers() {
                                let src = r#"builtins.readFile ./server.key"#;
                                let reports =
                                    nix_lint_core::lint_file(&make_registry(), src).unwrap();
                                assert!(!reports.is_empty());
                                assert_eq!(reports[0].code, 110);
                            }

                            #[test]
                            fn test_readfile_password_triggers() {
                                let src = r#"builtins.readFile ./passwords.txt"#;
                                let reports =
                                    nix_lint_core::lint_file(&make_registry(), src).unwrap();
                                assert!(!reports.is_empty());
                                assert_eq!(reports[0].code, 110);
                            }

                            #[test]
                            fn test_readfile_normal_no_trigger() {
                                let src = r#"builtins.readFile ./flake.nix"#;
                                let reports =
                                    nix_lint_core::lint_file(&make_registry(), src).unwrap();
                                assert!(reports.is_empty());
                            }

                            #[test]
                            fn test_readfile_nixpkgs_no_trigger() {
                                let src = r#"builtins.readFile "${nixpkgs}/default.nix""#;
                                let reports =
                                    nix_lint_core::lint_file(&make_registry(), src).unwrap();
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
