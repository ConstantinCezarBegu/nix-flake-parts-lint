//! nix-lint: a static analyzer for Nix configurations.
//!
//! Ported from the shell-based rules in nix/rules/ to Rust with the rnix parser.

mod registry;

use std::path::PathBuf;
use std::fs;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: nix-lint <directory>");
        std::process::exit(1);
    }

    let dir = PathBuf::from(&args[1]);
    let registry = registry::build_registry();
    let mut found_issues = false;

    let mut nix_files: Vec<(PathBuf, String)> = Vec::new();
    walk_nix_files(&dir, &registry, &mut found_issues, &mut nix_files);

    if found_issues {
        std::process::exit(1);
    }
}

fn walk_nix_files(dir: &std::path::Path, registry: &nix_lint_core::LintRegistry, found_issues: &mut bool, nix_files: &mut Vec<(PathBuf, String)>) {
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
            if file_name.starts_with('.') || file_name == "target" || file_name == "result" || file_name == ".git" || file_name == "secrets" || file_name == "test-fixtures" || file_name == "tests" {
                continue;
            }
            walk_nix_files(&path, registry, found_issues, nix_files);
        } else if path.extension().map_or(false, |ext| ext == "nix") {
            lint_file(&path, registry, found_issues, nix_files);
        }
    }
}

fn lint_file(path: &std::path::Path, registry: &nix_lint_core::LintRegistry, found_issues: &mut bool, nix_files: &mut Vec<(PathBuf, String)>) {
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return,
    };

    nix_files.push((path.to_path_buf(), src.clone()));

    match nix_lint_core::lint_file(registry, &src) {
        Ok(reports) => {
            for diag in reports.iter() {
                let d = diag.diagnostics.first().unwrap();
                let start = d.at.start().into();
                let line = src[..start].lines().count() + 1;
                let col = match src[..start].lines().last() {
                    Some(l) => l.chars().count(),
                    None => 0,
                };
                let severity = match diag.severity {
                    nix_lint_core::Severity::Warn => "WARN",
                    nix_lint_core::Severity::Error => "ERROR",
                    nix_lint_core::Severity::Hint => "HINT",
                };
                eprintln!(
                    "{}:{}:{} [{}] {} (note: {})",
                    path.display(),
                    line,
                    col + 1,
                    severity,
                    d.message,
                    diag.note
                );
            }
            if !reports.is_empty() {
                *found_issues = true;
            }
        }
        Err(err) => {
            let report = nix_lint_core::Report::from_parse_err(&err);
            if let Some(range) = report.total_range() {
                let line = src[..range.start().into()].lines().count() + 1;
                eprintln!(
                    "{}:{}: ERROR: {} (syntax error)",
                    path.display(),
                    line,
                    report.note
                );
            } else {
                eprintln!(
                    "{}: ERROR: {}",
                    path.display(),
                    report.note
                );
            }
            *found_issues = true;
        }
    }

    let file_reports = registry.validate_file(path, &src);
    for report in file_reports {
        let severity = match report.severity {
            nix_lint_core::Severity::Warn => "WARN",
            nix_lint_core::Severity::Error => "ERROR",
            nix_lint_core::Severity::Hint => "HINT",
        };
        eprintln!(
            "{} [{}] {} (note: {})",
            report.file,
            severity,
            report.message,
            report.note
        );
        *found_issues = true;
    }
}
