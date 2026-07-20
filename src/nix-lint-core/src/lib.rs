//! Core types and traits for the nix-flake-parts linter.

use std::{collections::HashMap, path::Path};

use rnix::parser::ParseError;

// ── Re-exports for downstream crates ────────────────────────────────────────

pub use rnix;
pub use rnix::{SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
pub use rowan;
pub use rowan::ast::AstNode;

// ── Severity ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warn,
    Error,
    Hint,
}

// ── Report / Diagnostic ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[must_use]
pub struct Report {
    pub note: &'static str,
    pub code: u32,
    pub severity: Severity,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct Diagnostic {
    pub message: String,
    pub at: TextRange,
}

impl Report {
    pub fn from_parse_err(err: &ParseError) -> Self {
        Self {
            note: "Failed to parse Nix file",
            code: 0,
            severity: Severity::Error,
            diagnostics: vec![Diagnostic {
                message: err.to_string(),
                at: TextRange::empty(0.into()),
            }],
        }
    }

    pub fn new(note: &'static str, code: u32, severity: Severity) -> Self {
        Self {
            note,
            code,
            severity,
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostic(mut self, at: TextRange, message: impl Into<String>) -> Self {
        self.diagnostics.push(Diagnostic {
            message: message.into(),
            at,
        });
        self
    }

    pub fn total_range(&self) -> Option<TextRange> {
        self.diagnostics.first().map(|_| {
            TextRange::new(
                self.diagnostics
                    .iter()
                    .map(|d| d.at.start())
                    .min()
                    .unwrap_or_default(),
                self.diagnostics
                    .iter()
                    .map(|d| d.at.end())
                    .max()
                    .unwrap_or_default(),
            )
        })
    }
}

// ── Rule trait ──────────────────────────────────────────────────────────────

pub trait Rule: Send + Sync {
    fn validate(&self, node: &SyntaxElement) -> Option<Report>;
}

// ── Metadata trait ──────────────────────────────────────────────────────────

pub trait Metadata: Rule {
    fn name(&self) -> &'static str;
    fn note(&self) -> &'static str;
    fn code(&self) -> u32;
    fn severity(&self) -> Severity {
        Severity::Warn
    }
    fn match_with(&self, kind: &SyntaxKind) -> bool;
    fn match_kind(&self) -> Vec<SyntaxKind>;
    fn report(&self) -> Report {
        Report::new(self.note(), self.code(), self.severity())
    }
}

// ── Explain trait ───────────────────────────────────────────────────────────

pub trait Explain: Metadata {
    fn explanation(&self) -> &'static str;
}

// ── LintRegistry ────────────────────────────────────────────────────────────

pub struct LintRegistry {
    lints: Vec<Box<dyn Lint>>,
    index: HashMap<SyntaxKind, Vec<usize>>,
    file_level_rules: Vec<Box<dyn FileLevelRule>>,
}

impl Default for LintRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LintRegistry {
    pub fn new() -> Self {
        Self {
            lints: Vec::new(),
            index: HashMap::new(),
            file_level_rules: Vec::new(),
        }
    }

    pub fn register(&mut self, lint: Box<dyn Lint>) {
        let indices = lint.match_kind();
        for kind in indices {
            let idx = self.lints.len();
            self.index.entry(kind).or_default().push(idx);
        }
        self.lints.push(lint);
    }

    pub fn register_file_level(&mut self, rule: Box<dyn FileLevelRule>) {
        self.file_level_rules.push(rule);
    }

    pub fn lints(&self) -> &[Box<dyn Lint>] {
        &self.lints
    }

    pub fn file_level_rules(&self) -> &[Box<dyn FileLevelRule>] {
        &self.file_level_rules
    }

    pub fn for_kind(&self, kind: &SyntaxKind) -> &[usize] {
        self.index.get(kind).map_or(&[], |v| v.as_slice())
    }

    pub fn validate_file(&self, path: &Path, content: &str) -> Vec<FileLevelReport> {
        let mut reports = Vec::new();
        for rule in &self.file_level_rules {
            if let Some(report) = rule.validate_file(path, content) {
                reports.push(report);
            }
        }
        reports
    }

    pub fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport> {
        let mut reports = Vec::new();
        for rule in &self.file_level_rules {
            reports.extend(rule.validate_project(files));
        }
        reports
    }
}

// ── Lint trait (combines Metadata + Rule + Explain) ─────────────────────────
// Each rule struct implements this via the #[lint] proc-macro
pub trait Lint: Metadata + Rule + Explain {
    fn as_rule(&self) -> &dyn Rule;
}

// ── Public entry point ──────────────────────────────────────────────────────

pub fn lint_file(registry: &LintRegistry, src: &str) -> Result<Vec<Report>, ParseError> {
    let parsed = rnix::Root::parse(src);
    let errors = parsed.errors();
    if !errors.is_empty() {
        return Err(errors[0].clone());
    }
    let root = parsed.syntax();
    let mut reports: HashMap<u32, (Severity, Vec<Diagnostic>)> = HashMap::new();

    walk_node(registry, &root, &mut reports);

    let mut result: Vec<Report> = reports
        .into_iter()
        .map(|(code, (severity, diagnostics))| {
            // Find the lint with this code to get the note
            let note = registry
                .lints()
                .iter()
                .find(|l| l.code() == code)
                .map(|l| l.note())
                .unwrap_or("unknown");
            Report {
                note,
                code,
                severity,
                diagnostics,
            }
        })
        .collect();
    result.sort_by_key(|r| r.code);
    Ok(result)
}

// ── File-level rules ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[must_use]
pub struct FileLevelReport {
    pub file: String,
    pub message: String,
    pub note: &'static str,
    pub code: u32,
    pub severity: Severity,
}

pub trait FileLevelRule: Send + Sync {
    fn code(&self) -> u32;
    fn name(&self) -> &'static str;
    fn severity(&self) -> Severity {
        Severity::Warn
    }
    fn note(&self) -> &'static str;
    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport>;
    fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport>;
}

// ── Walk helpers ────────────────────────────────────────────────────────────

fn walk_node(
    registry: &LintRegistry,
    node: &SyntaxNode,
    out: &mut HashMap<u32, (Severity, Vec<Diagnostic>)>,
) {
    for child in node.children_with_tokens() {
        run_node(registry, &child, out);
        if let SyntaxElement::Node(n) = &child {
            walk_node(registry, n, out);
        }
    }
}

fn run_node(
    registry: &LintRegistry,
    node: &SyntaxElement,
    out: &mut HashMap<u32, (Severity, Vec<Diagnostic>)>,
) {
    let kind = node.kind();
    for &idx in registry.for_kind(&kind) {
        let lint = &registry.lints()[idx];
        let code = lint.code();
        let severity = lint.severity();
        if let Some(report) = lint.as_rule().validate(node) {
            out.entry(code)
                .or_insert_with(|| (severity, Vec::new()))
                .1
                .extend(report.diagnostics);
        }
    }
}
