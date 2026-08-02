use std::collections::HashMap;
use std::path::Path;

use nix_lint_core::{FileLevelReport, FileLevelRule, Severity};
use regex::Regex;

pub struct ModuleImports;

impl ModuleImports {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ModuleImports {
    fn default() -> Self {
        Self::new()
    }
}

impl FileLevelRule for ModuleImports {
    fn code(&self) -> u32 {
        120
    }
    fn name(&self) -> &'static str {
        "module-imports"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn note(&self) -> &'static str {
        "Unimported file(s) found in module directory."
    }

    fn validate_file(&self, _path: &Path, _content: &str) -> Option<FileLevelReport> {
        None
    }

 fn validate_project(&self, files: &[(String, String)]) -> Vec<FileLevelReport> {
        let mut reports = Vec::new();

        // Collect all default.nix files (except flake.nix and hosts/default.nix)
        let all_default_nix: Vec<(String, String, String)> = files
            .iter()
            .filter(|(path, _)| {
                path.ends_with("default.nix")
                    && !path.ends_with("hosts/default.nix")
                    && !path.ends_with("flake.nix")
            })
            .map(|(path, content)| {
                let dir = Path::new(path)
                    .parent()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                (dir, path.clone(), content.clone())
            })
            .collect();

        // Build a map from directory -> content for all default.nix files
        let all_dirs: HashMap<String, String> = all_default_nix
            .iter()
            .map(|(dir, _, content)| (dir.clone(), content.clone()))
            .collect();

        // Build a list of (dir, imports_map) for directories that are siblings of module imports
        let module_check_dirs: Vec<(String, HashMap<String, String>)> = all_dirs
            .iter()
            .filter(|(dir, _)| is_sibling_of_module(dir, &all_default_nix))
            .map(|(dir, content)| {
                let imports = extract_imports(content);
                (dir.clone(), imports)
            })
            .collect();

        for (module_dir, imports_map) in &module_check_dirs {
            let module_default_content = all_default_nix
                .iter()
                .find(|(d, _, _)| d == module_dir)
                .map(|(_, _, c)| c.clone())
                .unwrap_or_default();
            // Collect leaf files from the actual filesystem in the module directory
            let leaf_files = collect_leaf_files(module_dir);

            for (rel_path, file_name) in &leaf_files {
                let base_name = rel_path
                    .trim_end_matches(".nix")
                    .trim_start_matches("./");
                let is_locally_imported = imports_map.contains_key(base_name)
                    || is_imported_in_content(&file_name, &module_default_content);
                let is_parent_imported = all_dirs.iter().any(|(parent_dir, parent_content)| {
                    if parent_dir == module_dir {
                        return false;
                    }
                    let parent_imports = extract_imports(parent_content);
                    let check_path = format!("{}/{}", parent_dir, base_name);
                    parent_imports.contains_key(base_name)
                        || parent_imports.keys().any(|import_key| {
                            check_path.contains(&format!("{}/", import_key))
                                || check_path.contains(import_key)
                        })
                });
                if !is_locally_imported && !is_parent_imported
                {
                    let import_path = if file_name.ends_with(".nix") && file_name != "default.nix" {
                        format!("./{}", file_name)
                    } else {
                        format!("./{}", base_name)
                    };
                    reports.push(FileLevelReport {
                        file: format!("{}/{}", module_dir, file_name),
                        message: format!(
                            "{} is not imported in default.nix (should be: {})",
                            file_name, import_path
                        ),
                        note: self.note(),
                        code: self.code(),
                        severity: self.severity(),
                    });
                }
            }
        }

        reports
    }
}

fn is_sibling_of_module(dir: &str, all_default_nix: &[(String, String, String)]) -> bool {
    // Check if dir's default.nix is a sibling of any import from any module default.nix
    for (_module_dir, _module_path, content) in all_default_nix {
        if !content.contains("flake.modules") {
            continue;
        }
        let imports = extract_imports(content);
        if is_sibling_of_import(dir, &imports) {
            return true;
        }
    }
    // Also check if this dir IS a module directory (i.e., dir's default.nix has flake.modules)
    for (_module_dir, module_path, content) in all_default_nix {
        if content.contains("flake.modules") {
            let module_dir = Path::new(&module_path)
                .parent()
                .unwrap()
                .to_string_lossy()
                .to_string();
            if dir == module_dir {
                return true;
            }
        }
    }
    false
}

fn is_imported_in_content(file_name: &str, content: &str) -> bool {
    let base_name = file_name.trim_end_matches(".nix");
    if content.contains(&format!("./{}", file_name))
        || content.contains(&format!("./{}", base_name))
        || content.contains(&format!("./{}", file_name.trim_end_matches('/')))
    {
        return true;
    }
    let re = Regex::new(r"\bimport\b").unwrap();
    for import_match in re.find_iter(content) {
        let import_start = import_match.start();
        let snippet = &content[import_start..];
        let snippet_end = std::cmp::min(snippet.len(), 200);
        let snippet = &snippet[..snippet_end];
        if snippet.contains(base_name) {
            return true;
        }
    }
    false
}

fn is_sibling_of_import(dir: &str, imports_map: &HashMap<String, String>) -> bool {
    for (import_base, _) in imports_map {
        // Check if the directory is directly the imported item
        if dir.ends_with(&format!("/{}", import_base)) || dir == import_base {
            return true;
        }
        // Check if the directory is the parent of an imported file (e.g., dir is parent of ./fish.nix)
        if import_base.ends_with(".nix") {
            let import_dir = import_base.trim_end_matches(".nix").rsplit('/').next().unwrap_or("");
            let dir_parent = dir.rsplit('/').next().unwrap_or("");
            if import_dir == "" && dir_parent == dir {
                return true;
            }
        }
    }
    false
}

fn extract_imports(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    let re = Regex::new(r"imports\s*=\s*\[([^]]+)\];").unwrap();
    if let Some(cap) = re.captures(content) {
        let imports_block = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        for line in imports_block.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(path) = trimmed.strip_prefix("./") {
                let base_name = path
                    .trim_end_matches(".nix")
                    .trim_end_matches('/');
                map.insert(base_name.to_string(), path.to_string());
            }
        }
    }

    map
}

fn collect_leaf_files(dir: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        if path.is_file() {
            if file_name == "default.nix" {
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "nix") {
                let rel = format!("./{}", file_name);
                files.push((rel, file_name));
            }
        } else if path.is_dir() {
            let default_nix = path.join("default.nix");
            if default_nix.is_file() {
                let rel = format!("./{}", file_name);
                files.push((rel, file_name));
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/test/{}", name))
    }

    #[test]
    fn test_module_imports_all_files_imported_no_report() {
        let rule = ModuleImports::new();
        let dir = "/tmp/test_module_imports_0";
        let _ = std::fs::create_dir_all(dir);
        std::fs::write(format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./fish.nix
    ./ssh.nix
  ];
  flake.modules.darwin.base = { ... }: {
    imports = [ self.modules.darwin.fish self.modules.darwin.ssh ];
  };
}"#).unwrap();
        std::fs::write(format!("{}/fish.nix", dir), "{}").unwrap();
        std::fs::write(format!("{}/ssh.nix", dir), "{}").unwrap();

        let files: Vec<(String, String)> = vec![
            (format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./fish.nix
    ./ssh.nix
  ];
  flake.modules.darwin.base = { ... }: {
    imports = [ self.modules.darwin.fish self.modules.darwin.ssh ];
  };
}"#.to_string()),
            (format!("{}/fish.nix", dir), "{}".to_string()),
            (format!("{}/ssh.nix", dir), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty(), "Expected no reports, got: {:?}", reports);
    }

   #[test]
    fn test_module_imports_missing_leaf_file_report() {
        let rule = ModuleImports::new();
        let dir = "/tmp/test_module_imports_1";
        let _ = std::fs::create_dir_all(dir);
        std::fs::write(format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./fish.nix
    ./ssh.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#).unwrap();
        std::fs::write(format!("{}/fish.nix", dir), "{}").unwrap();
        std::fs::write(format!("{}/ssh.nix", dir), "{}").unwrap();
        std::fs::write(format!("{}/stylix.nix", dir), "{}").unwrap();

        let files: Vec<(String, String)> = vec![
            (format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./fish.nix
    ./ssh.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#.to_string()),
            (format!("{}/fish.nix", dir), "{}".to_string()),
            (format!("{}/ssh.nix", dir), "{}".to_string()),
            (format!("{}/stylix.nix", dir), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert_eq!(reports.len(), 1, "Expected exactly 1 report");
        assert_eq!(reports[0].code, 120);
        assert_eq!(reports[0].severity, Severity::Error);
        assert!(reports[0].message.contains("stylix.nix"));
    }

    #[test]
    fn test_module_imports_directory_with_default_nix_no_report() {
        let rule = ModuleImports::new();
        let dir = "/tmp/test_module_imports_4";
        let aero_dir = format!("{}/aerospace", dir);
        let _ = std::fs::create_dir_all(&aero_dir);
        std::fs::write(format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./aerospace
    ./fish.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#).unwrap();
        std::fs::write(format!("{}/default.nix", aero_dir), "{}").unwrap();
        std::fs::write(format!("{}/fish.nix", dir), "{}").unwrap();

        let files: Vec<(String, String)> = vec![
            (format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./aerospace
    ./fish.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#.to_string()),
            (format!("{}/default.nix", aero_dir), "{}".to_string()),
            (format!("{}/fish.nix", dir), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty(), "Expected no reports, got: {:?}", reports);
    }

   #[test]
    fn test_module_imports_nested_leaf_not_imported_report() {
        let rule = ModuleImports::new();
        let dir = "/tmp/test_module_imports_2";
        let aero_dir = format!("{}/aerospace", dir);
        let _ = std::fs::create_dir_all(&aero_dir);
        std::fs::write(format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./aerospace
    ./fish.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#).unwrap();
        std::fs::write(format!("{}/default.nix", aero_dir), "{}").unwrap();
        std::fs::write(format!("{}/fish.nix", dir), "{}").unwrap();
        std::fs::write(format!("{}/config.nix", aero_dir), "{}").unwrap();

        let files: Vec<(String, String)> = vec![
            (format!("{}/default.nix", dir), r#"{ self, ... }: {
  imports = [
    ./aerospace
    ./fish.nix
  ];
  flake.modules.darwin.base = { ... }: {};
}"#.to_string()),
            (format!("{}/default.nix", aero_dir), "{}".to_string()),
            (format!("{}/fish.nix", dir), "{}".to_string()),
            (format!("{}/config.nix", aero_dir), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert_eq!(reports.len(), 1, "Expected exactly 1 report");
        assert!(reports[0].message.contains("config.nix"));
    }

    #[test]
    fn test_extract_imports_parses_correctly() {
        let content = r#"{ self, ... }: {
  imports = [
    ./fish.nix
    ./ssh.nix
    ./stylix.nix
    ./aerospace
  ];
  flake.modules.darwin.base = { ... }: {};
}"#;
        let imports = extract_imports(content);
        assert!(imports.contains_key("fish"));
        assert!(imports.contains_key("ssh"));
        assert!(imports.contains_key("stylix"));
        assert!(imports.contains_key("aerospace"));
        assert_eq!(imports.len(), 4);
    }

   #[test]
    fn test_no_imports_block_no_report() {
        let rule = ModuleImports::new();
        let dir = "/tmp/test_module_imports_3";
        let _ = std::fs::create_dir_all(dir);
        std::fs::write(format!("{}/default.nix", dir), r#"{ self, ... }: {
  flake.modules.darwin.base = { ... }: {};
}"#).unwrap();
        std::fs::write(format!("{}/fish.nix", dir), "{}").unwrap();

        let files: Vec<(String, String)> = vec![
            (format!("{}/default.nix", dir), r#"{ self, ... }: {
  flake.modules.darwin.base = { ... }: {};
}"#.to_string()),
            (format!("{}/fish.nix", dir), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert_eq!(reports.len(), 1);
        assert!(reports[0].message.contains("fish.nix"));
    }

    #[test]
    fn test_module_dir_without_flake_modules_ignored() {
        let rule = ModuleImports::new();
        let files: Vec<(String, String)> = vec![
            (
                "/tmp/test/packages/default.nix".to_string(),
                r#"{ lib, pkgs, ... }: {
  packages.mytool = pkgs.writeShellScriptBin "mytool" "echo hello";
}"#
                    .to_string(),
            ),
            ("/tmp/test/packages/helper.nix".to_string(), "{}".to_string()),
        ];
        let reports = rule.validate_project(&files);
        assert!(reports.is_empty(), "Expected no reports for non-module dirs");
    }
}
