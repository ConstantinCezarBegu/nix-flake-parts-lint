//! Core types and traits for the nix-flake-parts linter.

use std::{collections::HashMap, path::Path};

use rnix::parser::ParseError;

// ── Re-exports for downstream crates ────────────────────────────────────────

pub use rnix;
pub use rnix::{SyntaxElement, SyntaxKind, SyntaxNode, TextRange};
pub use rowan;
pub use rowan::ast::AstNode;

// ── Severity ────────────────────────────────────────────────────────────────

/// Severity level of a lint diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Warning-level diagnostic.
    Warn,
    /// Error-level diagnostic.
    Error,
    /// Hint-level diagnostic.
    Hint,
}

// ── Report / Diagnostic ─────────────────────────────────────────────────────

/// A lint report containing one or more diagnostics for a single rule code.
#[derive(Debug, Clone)]
#[must_use]
pub struct Report {
    /// Human-readable note describing the lint rule.
    pub note: &'static str,
    /// Unique numeric code for the lint rule.
    pub code: u32,
    /// Severity level of the report.
    pub severity: Severity,
    /// List of diagnostics with messages and source ranges.
    pub diagnostics: Vec<Diagnostic>,
}

/// A single diagnostic with a message and source code range.
#[derive(Debug, Clone)]
#[must_use]
pub struct Diagnostic {
    /// Human-readable diagnostic message.
    pub message: String,
    /// Source code range where the diagnostic applies.
    pub at: TextRange,
}

impl Report {
    /// Create a report from a Nix parse error.
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

    /// Create a new empty report with the given note, code, and severity.
    pub fn new(note: &'static str, code: u32, severity: Severity) -> Self {
        Self {
            note,
            code,
            severity,
            diagnostics: Vec::new(),
        }
    }

    /// Add a diagnostic to this report at the given source range with the given message.
    pub fn diagnostic(mut self, at: TextRange, message: impl Into<String>) -> Self {
        self.diagnostics.push(Diagnostic {
            message: message.into(),
            at,
        });
        self
    }

    /// Compute the combined source range covering all diagnostics in this report.
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

/// Trait for lint rules that validate AST nodes.
pub trait Rule: Send + Sync {
    /// Validate the given syntax node and return a report if violations are found.
    fn validate(&self, node: &SyntaxElement) -> Option<Report>;
}

// ── Metadata trait ──────────────────────────────────────────────────────────

/// Trait providing metadata about a lint rule.
pub trait Metadata: Rule {
    /// Human-readable name of the lint rule.
    fn name(&self) -> &'static str;
    /// Brief note describing the rule's purpose.
    fn note(&self) -> &'static str;
    /// Unique numeric code for the rule.
    fn code(&self) -> u32;
    /// Default severity for the rule.
    fn severity(&self) -> Severity {
        Severity::Warn
    }
    /// Check if this rule applies to the given syntax kind.
    fn match_with(&self, kind: &SyntaxKind) -> bool;
    /// List of syntax kinds this rule matches.
    fn match_kind(&self) -> Vec<SyntaxKind>;
    /// Create a default report for this rule.
    fn report(&self) -> Report {
        Report::new(self.note(), self.code(), self.severity())
    }
}

// ── Explain trait ───────────────────────────────────────────────────────────

/// Trait for lint rules that provide a detailed explanation of the violation.
pub trait Explain: Metadata {
    /// Detailed explanation of what the rule checks and how to fix violations.
    fn explanation(&self) -> &'static str;
}

// ── LintRegistry ────────────────────────────────────────────────────────────

/// Registry that stores lint rules and file-level rules, enabling fast lookup by syntax kind.
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
    /// Create a new empty lint registry.
    pub fn new() -> Self {
        Self {
            lints: Vec::new(),
            index: HashMap::new(),
            file_level_rules: Vec::new(),
        }
    }

    /// Register a lint rule in the registry.
    pub fn register(&mut self, lint: Box<dyn Lint>) {
        let indices = lint.match_kind();
        for kind in indices {
            let idx = self.lints.len();
            self.index.entry(kind).or_default().push(idx);
        }
        self.lints.push(lint);
    }

    /// Register a file-level rule in the registry.
    pub fn register_file_level(&mut self, rule: Box<dyn FileLevelRule>) {
        self.file_level_rules.push(rule);
    }

    /// Return all registered lint rules.
    pub fn lints(&self) -> &[Box<dyn Lint>] {
        &self.lints
    }

    /// Return all registered file-level rules.
    pub fn file_level_rules(&self) -> &[Box<dyn FileLevelRule>] {
        &self.file_level_rules
    }

    /// Return indices of lint rules that match the given syntax kind.
    pub fn for_kind(&self, kind: &SyntaxKind) -> &[usize] {
        self.index.get(kind).map_or(&[], |v| v.as_slice())
    }

    /// Validate a single file against all file-level rules.
    pub fn validate_file(&self, path: &Path, content: &str) -> Vec<FileLevelReport> {
        let mut reports = Vec::new();
        for rule in &self.file_level_rules {
            if let Some(report) = rule.validate_file(path, content) {
                reports.push(report);
            }
        }
        reports
    }

    /// Validate a project (multiple files) against all file-level rules.
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
/// Combined trait for lint rules that requires Metadata, Rule, and Explain implementations.
pub trait Lint: Metadata + Rule + Explain {
    /// Downcast to the Rule trait object.
    fn as_rule(&self) -> &dyn Rule;
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Lint a single Nix source file against all registered rules.
///
/// Returns a vector of reports, one per violated rule.
/// Returns an error if the source contains parse errors.
pub fn lint_file(registry: &LintRegistry, src: &str) -> Result<Vec<Report>, ParseError> {
    let parsed = rnix::Root::parse(src);
    let errors = parsed.errors();
    if let Some(first_error) = errors.first() {
        return Err(first_error.clone());
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

/// A report from a file-level rule validation.
#[derive(Debug, Clone)]
#[must_use]
pub struct FileLevelReport {
    /// Path to the file being reported.
    pub file: String,
    /// Human-readable message describing the issue.
    pub message: String,
    /// Brief note describing the rule.
    pub note: &'static str,
    /// Unique numeric code for the rule.
    pub code: u32,
    /// Severity level of the report.
    pub severity: Severity,
}

/// Trait for rules that validate entire files or projects (not individual AST nodes).
pub trait FileLevelRule: Send + Sync {
    /// Unique numeric code for the rule.
    fn code(&self) -> u32;
    /// Human-readable name of the rule.
    fn name(&self) -> &'static str;
    /// Default severity for the rule.
    fn severity(&self) -> Severity {
        Severity::Warn
    }
    /// Brief note describing the rule's purpose.
    fn note(&self) -> &'static str;
    /// Validate a single file and return a report if violations are found.
    fn validate_file(&self, path: &Path, content: &str) -> Option<FileLevelReport>;
    /// Validate a project (multiple files) and return reports for any violations.
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
