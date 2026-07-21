use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf()
}

fn lint_command() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nix_flake_parts_lint"));
    cmd.current_dir(project_root());
    cmd
}

fn create_test_dir(name: &str) -> PathBuf {
    let test_dir = std::env::temp_dir().join(format!("nix-lint-test-{}", name));
    let hosts_dir = test_dir.join("hosts");
    fs::create_dir_all(&hosts_dir).expect("failed to create test dir");
    hosts_dir.join("test.nix")
}

fn cleanup_test_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

fn run_test(content: &str, _expect_success: bool) -> (bool, String) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let test_file = create_test_dir(&id.to_string());
    fs::write(&test_file, content).expect("failed to write test file");

    let output = lint_command()
        .arg(&test_file)
        .output()
        .expect("failed to execute nix-lint");

    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    cleanup_test_dir(&test_file);

    (success, stderr)
}

#[test]
fn lint_rec_attrset_returns_failure() {
    let content = r#"rec {
  foo = bar;
  bar = 42;
}"#;
    let (success, stderr) = run_test(content, false);
    assert!(!success, "Expected failure but got success");
    assert!(
        stderr.contains("rec") || stderr.contains("WARN") || stderr.contains("ERROR"),
        "Expected lint output but got: {}",
        stderr
    );
}

#[test]
fn lint_clean_file_returns_success() {
    let content = r#"{ lib, ... }: {
  options.myOption = lib.mkOption {
    description = "A test option";
    type = lib.types.bool;
  };

  config = {
    myOption = false;
  };
}"#;
    let (success, stderr) = run_test(content, true);
    assert!(
        success,
        "Expected success but got failure. stderr: {}",
        stderr
    );
}

#[test]
fn lint_with_pkgs_returns_failure() {
    let content = r#"{ lib, pkgs, ... }:
let
  hello = with pkgs; hello;
in {
  options.test = lib.mkOption {
    description = "A test option";
    type = lib.types.bool;
  };
}"#;
    let (success, stderr) = run_test(content, false);
    assert!(!success, "Expected failure but got success");
    assert!(
        stderr.contains("with") || stderr.contains("WARN"),
        "Expected with-pkgs lint output but got: {}",
        stderr
    );
}

#[test]
fn lint_list_command_succeeds() {
    let output = lint_command()
        .arg(".")
        .arg("list")
        .output()
        .expect("failed to execute nix-lint list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Available lint rules:"));
    assert!(stdout.contains("File-level rules:"));
}

#[test]
fn lint_explain_command_succeeds() {
    let output = lint_command()
        .arg(".")
        .arg("explain")
        .arg("no-rec")
        .output()
        .expect("failed to execute nix-lint explain");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rule: no-rec"));
    assert!(stdout.contains("Code:"));
    assert!(stdout.contains("Severity:"));
}

#[test]
fn lint_explain_unknown_rule_fails() {
    let output = lint_command()
        .arg(".")
        .arg("explain")
        .arg("nonexistent-rule")
        .output()
        .expect("failed to execute nix-lint explain");

    assert!(!output.status.success());
}
