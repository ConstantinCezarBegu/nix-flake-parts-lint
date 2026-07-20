//! nix-lint: a static analyzer for Nix configurations.
//!
//! Ported from the shell-based rules in nix/rules/ to Rust with the rnix parser.

#![allow(clippy::collapsible_if)]

mod registry;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use clap::{Parser, Subcommand};
use nix_lint_core::LintRegistry;

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
    disabled: HashSet<String>,
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
    let mut config = ConfigFile::default();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("disabled = [") {
            if let Some(end) = value.find(']') {
                let items = &value[..end];
                for item in items.split(',') {
                    let item = item.trim().trim_matches('"').trim_matches('\'');
                    if !item.is_empty() {
                        config.disabled.insert(item.to_string());
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
    let mut found_issues = false;

    let mut nix_files: Vec<(PathBuf, String)> = Vec::new();
    walk_nix_files(
        &cli.path,
        &registry,
        &mut found_issues,
        &mut nix_files,
        config,
    );

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

fn walk_nix_files(
    dir: &std::path::Path,
    registry: &LintRegistry,
    found_issues: &mut bool,
    nix_files: &mut Vec<(PathBuf, String)>,
    config: &ConfigFile,
) {
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
            walk_nix_files(&path, registry, found_issues, nix_files, config);
        } else if path.extension().is_some_and(|ext| ext == "nix") {
            lint_file(&path, registry, found_issues, nix_files, config);
        }
    }
}

fn lint_file(
    path: &std::path::Path,
    registry: &LintRegistry,
    found_issues: &mut bool,
    nix_files: &mut Vec<(PathBuf, String)>,
    config: &ConfigFile,
) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return,
    };

    nix_files.push((path.to_path_buf(), src.clone()));

    match nix_lint_core::lint_file(registry, &src) {
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
                print_reports(path, &src, report, severity);
            }
            if !reports.is_empty() {
                *found_issues = true;
            }
        }
        Err(err) => {
            let report = nix_lint_core::Report::from_parse_err(&err);
            print_parse_error(path, &src, &report);
            *found_issues = true;
        }
    }

    let file_reports = registry.validate_file(path, &src);
    for report in &file_reports {
        if config.disabled.contains(&report.code.to_string()) {
            continue;
        }
        let severity = match report.severity {
            nix_lint_core::Severity::Warn => "WARN",
            nix_lint_core::Severity::Error => "ERROR",
            nix_lint_core::Severity::Hint => "HINT",
        };
        eprintln!(
            "{} [{}] {} (note: {})",
            report.file, severity, report.message, report.note
        );
        *found_issues = true;
    }
}

fn print_reports(
    path: &std::path::Path,
    src: &str,
    report: &nix_lint_core::Report,
    severity: &str,
) {
    if report.diagnostics.is_empty() {
        return;
    }

    for diag in &report.diagnostics {
        let start = usize::from(diag.at.start());
        let line = src[..start].lines().count() + 1;
        let col = match src[..start].lines().last() {
            Some(l) => l.chars().count(),
            None => 0,
        };

        eprintln!(
            "{}:{}:{} [{}] {} (note: {})",
            path.display(),
            line,
            col + 1,
            severity,
            diag.message,
            report.note
        );
    }
}

fn print_parse_error(path: &std::path::Path, src: &str, report: &nix_lint_core::Report) {
    if let Some(range) = report.total_range() {
        let start = usize::from(range.start());
        let line = src[..start].lines().count() + 1;
        eprintln!(
            "{}:{}: ERROR: {} (syntax error)",
            path.display(),
            line,
            report.note
        );
    } else {
        eprintln!("{}: ERROR: {}", path.display(), report.note);
    }
}
