//! nix-lint: a static analyzer for Nix configurations.
//!
//! Ported from the shell-based rules in nix/rules/ to Rust with the rnix parser.

#![allow(clippy::collapsible_if)]

mod registry;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};
use nix_lint_core::LintRegistry;
use rayon::prelude::*;

/// nix-lint: static analysis for Nix flake-parts configurations
#[derive(Parser, Debug)]
#[command(name = "nix-lint", version, about)]
struct Cli {
    /// Path to directory or file to lint
    #[arg(name = "path")]
    path: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all available lint rules
    List,
    /// Show explanation for a specific lint rule
    Explain {
        /// Name of the lint rule (e.g., "no-rec")
        #[arg(name = "name")]
        name: String,
    },
}

#[derive(Debug, Default)]
struct ConfigFile {
    disabled: BTreeSet<String>,
}

static CONFIG: OnceLock<ConfigFile> = OnceLock::new();

fn get_config(root: &std::path::Path) -> &'static ConfigFile {
    CONFIG.get_or_init(|| search_config_file(root))
}

fn search_config_file(start: &std::path::Path) -> ConfigFile {
    let mut current = start.to_path_buf();

    for _ in 0..20 {
        let config_path = current.join(".nix-lint.toml");
        if config_path.is_file() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                return parse_config(&content);
            }
        }

        match current.parent() {
            Some(parent) if parent.to_string_lossy() != current.to_string_lossy() => {
                current = parent.to_path_buf();
            }
            _ => break,
        }
    }

    ConfigFile::default()
}

fn parse_config(content: &str) -> ConfigFile {
    let parsed: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: failed to parse .nix-lint.toml: {}", e);
            return ConfigFile::default();
        }
    };

    let mut config = ConfigFile::default();

    if let Some(disabled) = parsed.get("disabled") {
        if let Some(rules) = disabled.get("rules") {
            if let Some(array) = rules.as_array() {
                for value in array {
                    if let Some(name) = value.as_str() {
                        config.disabled.insert(name.to_string());
                    }
                }
            }
        }
    }

    config
}

fn main() {
    let cli = Cli::parse();
    let config = get_config(&cli.path);

    match cli.command {
        Some(Commands::List) => {
            list_lints();
            return;
        }
        Some(Commands::Explain { name }) => {
            explain_lint(&name);
            return;
        }
        None => {}
    }

    let registry = registry::build_registry();

    let nix_files = collect_nix_files(&cli.path);
    let found_issues = lint_files_parallel(&nix_files, &registry, config);

    if found_issues {
        std::process::exit(1);
    }
}

fn list_lints() {
    let registry = registry::build_registry();

    println!("Available lint rules:");
    println!();

    for lint in registry.lints() {
        let explain: &dyn nix_lint_core::Explain = lint.as_ref();
        println!("  {} - {}", lint.code(), lint.name());
        println!("    {}", lint.note());
        let explanation = explain.explanation();
        let first_line = explanation.lines().next().unwrap_or("");
        if !first_line.is_empty() {
            println!("    {}", first_line);
        }
        println!();
    }

    println!("File-level rules:");
    println!();

    for rule in registry.file_level_rules() {
        println!("  {} - {}", rule.code(), rule.name());
        println!("    {}", rule.note());
        println!();
    }
}

fn explain_lint(name: &str) {
    let registry = registry::build_registry();

    for lint in registry.lints() {
        if lint.name() == name {
            let explain: &dyn nix_lint_core::Explain = lint.as_ref();
            println!("Rule: {}", lint.name());
            println!("Code: {}", lint.code());
            println!("Severity: {:?}", lint.severity());
            println!("Note: {}", lint.note());
            println!();
            println!("Explanation:");
            for line in explain.explanation().lines() {
                println!("  {}", line);
            }
            return;
        }
    }

    eprintln!("Error: unknown lint rule '{}'", name);
    std::process::exit(1);
}

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    content: String,
}

fn collect_nix_files(dir: &std::path::Path) -> Vec<FileEntry> {
    if dir.is_file() {
        if let Some(ext) = dir.extension() {
            if ext == "nix" {
                if let Ok(content) = fs::read_to_string(dir) {
                    return vec![FileEntry {
                        path: dir.to_path_buf(),
                        content,
                    }];
                }
            }
        }
        return Vec::new();
    }
    let mut files = Vec::new();
    collect_nix_files_inner(dir, &mut files);
    files
}

fn collect_nix_files_inner(dir: &std::path::Path, files: &mut Vec<FileEntry>) {
    if !dir.is_dir() {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();

        if path.is_dir() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name.starts_with('.')
                || file_name == "target"
                || file_name == "result"
                || file_name == ".git"
                || file_name == "secrets"
                || file_name == "test-fixtures"
                || file_name == "tests"
            {
                continue;
            }
            collect_nix_files_inner(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "nix") {
            if let Ok(content) = fs::read_to_string(&path) {
                files.push(FileEntry { path, content });
            }
        }
    }
}

fn lint_files_parallel(files: &[FileEntry], registry: &LintRegistry, config: &ConfigFile) -> bool {
    let found_issues = AtomicBool::new(false);

    let results: Vec<(Vec<String>, bool)> = files
        .par_iter()
        .map(|file| lint_file_messages(&file.path, &file.content, registry, config))
        .collect();

    for (messages, has_issues) in results {
        if has_issues {
            found_issues.store(true, Ordering::Relaxed);
        }
        for msg in messages {
            eprintln!("{}", msg);
        }
    }

    found_issues.load(Ordering::Relaxed)
}

#[must_use]
fn lint_file_messages(
    path: &std::path::Path,
    src: &str,
    registry: &LintRegistry,
    config: &ConfigFile,
) -> (Vec<String>, bool) {
    let mut messages = Vec::new();
    let mut has_issues = false;

    match nix_lint_core::lint_file(registry, src) {
        Ok(reports) => {
            for report in &reports {
                if config.disabled.contains(&report.code.to_string()) {
                    continue;
                }
                let severity = match report.severity {
                    nix_lint_core::Severity::Warn => "WARN",
                    nix_lint_core::Severity::Error => "ERROR",
                    nix_lint_core::Severity::Hint => "HINT",
                };
                let mut has_report_issues = false;
                for diag in &report.diagnostics {
                    has_issues = true;
                    has_report_issues = true;
                    let start = usize::from(diag.at.start());
                    let line = src[..start].lines().count() + 1;
                    let col = match src[..start].lines().last() {
                        Some(l) => l.chars().count(),
                        None => 0,
                    };
                    messages.push(format!(
                        "{}:{}:{} [{}] {} (note: {})",
                        path.display(),
                        line,
                        col + 1,
                        severity,
                        diag.message,
                        report.note
                    ));
                }
                if has_report_issues {
                    messages.push(String::new());
                }
            }
        }
        Err(err) => {
            has_issues = true;
            let report = nix_lint_core::Report::from_parse_err(&err);
            if let Some(range) = report.total_range() {
                let start = usize::from(range.start());
                let line = src[..start].lines().count() + 1;
                messages.push(format!(
                    "{}:{}: ERROR: {} (syntax error)",
                    path.display(),
                    line,
                    report.note
                ));
            } else {
                messages.push(format!("{}: ERROR: {}", path.display(), report.note));
            }
        }
    }

    let file_reports = registry.validate_file(path, src);
    for report in &file_reports {
        if config.disabled.contains(&report.code.to_string()) {
            continue;
        }
        let severity = match report.severity {
            nix_lint_core::Severity::Warn => "WARN",
            nix_lint_core::Severity::Error => "ERROR",
            nix_lint_core::Severity::Hint => "HINT",
        };
        messages.push(format!(
            "{} [{}] {} (note: {})",
            report.file, severity, report.message, report.note
        ));
    }

    (messages, has_issues)
}
