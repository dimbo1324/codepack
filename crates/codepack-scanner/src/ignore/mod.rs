//! `.exportignore` parsing and programmatic exclusion rules, ported from legacy
//! `services/export_ignore.py`. `should_skip_dir`/`should_skip_file` live in
//! [`rules`] with the exact legacy precedence; this module owns parsing, storage, and
//! the [`ScanOptions`] adapter between [`codepack_core::Config`] and the scanner.

pub(crate) mod pattern;
mod rules;

pub use pattern::GlobPattern;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub(crate) struct FilePattern {
    pub(crate) raw: String,
    pub(crate) matcher: GlobPattern,
}

impl FilePattern {
    fn new(raw: String) -> Self {
        let matcher = GlobPattern::new(&raw);
        Self { raw, matcher }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DirPattern {
    pub(crate) raw: String,
    pub(crate) matcher: GlobPattern,
}

impl DirPattern {
    fn new(raw: String) -> Self {
        let matcher = GlobPattern::new(&raw);
        Self { raw, matcher }
    }
}

/// Parsed `.exportignore` rules plus programmatic exclusion/always-include lists,
/// mirroring legacy `ExportIgnoreRules`. Predicates live in [`rules`].
#[derive(Debug, Clone, Default)]
pub struct ExportIgnoreRules {
    pub(crate) excluded_dirs: Vec<DirPattern>,
    pub(crate) excluded_files: Vec<FilePattern>,
    pub(crate) excluded_extensions: std::collections::HashSet<String>,
    pub(crate) always_include_files: Vec<FilePattern>,
    pub(crate) always_include_dirs: std::collections::HashSet<String>,
    pub source_file: Option<PathBuf>,
    pub loaded_rules: Vec<String>,
}

/// Scanner-facing settings, decoupled from `codepack_core::Config` beyond this one
/// conversion boundary. Values are taken as-is except where the `Config` accessor
/// normalizes them (`normalized_export_profile`, etc.).
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub extra_ignored_dirs: Vec<String>,
    pub custom_excluded_files: Vec<String>,
    pub custom_excluded_extensions: Vec<String>,
    pub always_include_files: Vec<String>,
    pub always_include_dirs: Vec<String>,
    pub export_profile: String,
    pub safe_export_mode: String,
    pub diff_export_mode: String,
    pub incremental_export_enabled: bool,
}

impl From<&codepack_core::config::Config> for ScanOptions {
    fn from(config: &codepack_core::config::Config) -> Self {
        Self {
            extra_ignored_dirs: config.extra_ignored_dirs.clone(),
            custom_excluded_files: config.custom_excluded_files.clone(),
            custom_excluded_extensions: config.custom_excluded_extensions.clone(),
            always_include_files: config.always_include_files.clone(),
            always_include_dirs: config.always_include_dirs.clone(),
            export_profile: config.normalized_export_profile().to_string(),
            safe_export_mode: config.normalized_safe_export_mode().to_string(),
            diff_export_mode: config.normalized_diff_export_mode().to_string(),
            incremental_export_enabled: config.incremental_export_enabled,
        }
    }
}

/// Serializable view of [`ExportIgnoreRules`], field order matching the legacy
/// `to_dict()` contract exactly (invariant I5): `source_file`, `loaded_rules`,
/// `excluded_dirs` (sorted), `excluded_files` (insertion order), `excluded_extensions`
/// (sorted), `always_include_files` (insertion order), `always_include_dirs` (sorted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesReport {
    pub source_file: Option<String>,
    pub loaded_rules: Vec<String>,
    pub excluded_dirs: Vec<String>,
    pub excluded_files: Vec<String>,
    pub excluded_extensions: Vec<String>,
    pub always_include_files: Vec<String>,
    pub always_include_dirs: Vec<String>,
}

/// Normalizes a `.exportignore`/config *file* rule value as it is stored: whitespace
/// trimmed, backslashes to forward slashes, casefolded (legacy
/// `add_file_rule`/`add_always_include_file`: `value.strip().replace("\\","/").casefold()`).
fn normalize_rule_value(value: &str) -> String {
    value.trim().replace('\\', "/").to_lowercase()
}

/// Normalizes a `.exportignore`/config *directory* rule value: backslashes to forward
/// slashes first, then leading/trailing slashes stripped, then casefolded — matching
/// legacy `_normalise_path`'s order exactly (`str(path).replace("\\","/").strip("/").casefold()`).
/// Order matters: stripping slashes before flipping separators would miss a
/// backslash-wrapped value such as `"\\private\\"`.
fn normalize_dir_value(value: &str) -> String {
    let replaced = value.replace('\\', "/");
    replaced.trim_matches('/').to_lowercase()
}

fn clean_rule_line(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return String::new();
    }
    let cut = match trimmed.find(" #") {
        Some(idx) => trimmed[..idx].trim(),
        None => trimmed,
    };
    cut.replace('\\', "/")
}

impl ExportIgnoreRules {
    pub fn from_project_and_config(source_root: &Path, options: &ScanOptions) -> Self {
        let mut rules = Self::default();

        let ignore_file = source_root.join(".exportignore");
        if ignore_file.is_file() {
            rules.source_file = Some(ignore_file.clone());
            // Legacy reads with `errors="replace"` and swallows any exception raised
            // while parsing the optional file; `from_utf8_lossy` gives the same
            // never-fails behavior for invalid encodings, and a missing-by-race or
            // unreadable file is treated the same as "no rules loaded".
            if let Ok(bytes) = std::fs::read(&ignore_file) {
                let content = String::from_utf8_lossy(&bytes);
                let lines: Vec<String> = content.lines().map(str::to_string).collect();
                rules.load_lines(&lines);
            }
        }

        for item in &options.custom_excluded_files {
            rules.add_file_rule(item);
        }
        for item in &options.custom_excluded_extensions {
            rules.add_extension_rule(item);
        }
        for item in &options.always_include_files {
            rules.add_always_include_file(item);
        }
        for item in &options.always_include_dirs {
            rules.add_always_include_dir(item);
        }
        rules
    }

    fn load_lines(&mut self, lines: &[String]) {
        for raw_line in lines {
            let rule = clean_rule_line(raw_line);
            if rule.is_empty() {
                continue;
            }
            self.loaded_rules.push(rule.clone());

            let include = rule.starts_with('!');
            let rule = if include {
                rule[1..].trim()
            } else {
                rule.as_str()
            };
            if rule.is_empty() {
                continue;
            }

            if let Some(target) = rule.strip_suffix('/') {
                if include {
                    self.add_always_include_dir(target);
                } else {
                    self.add_dir_rule(target);
                }
                continue;
            }

            if include {
                self.add_always_include_file(rule);
            } else {
                self.add_file_rule(rule);
            }
        }
    }

    pub fn add_dir_rule(&mut self, value: &str) {
        let normalized = normalize_dir_value(value);
        if !normalized.is_empty() {
            self.excluded_dirs.push(DirPattern::new(normalized));
        }
    }

    pub fn add_file_rule(&mut self, value: &str) {
        let normalized = normalize_rule_value(value);
        if !normalized.is_empty() {
            self.excluded_files.push(FilePattern::new(normalized));
        }
    }

    pub fn add_extension_rule(&mut self, value: &str) {
        let normalized = value.trim().trim_start_matches('.').to_lowercase();
        if !normalized.is_empty() {
            self.excluded_extensions.insert(normalized);
        }
    }

    pub fn add_always_include_file(&mut self, value: &str) {
        let normalized = normalize_rule_value(value);
        if !normalized.is_empty() {
            self.always_include_files.push(FilePattern::new(normalized));
        }
    }

    pub fn add_always_include_dir(&mut self, value: &str) {
        let normalized = normalize_dir_value(value);
        if !normalized.is_empty() {
            self.always_include_dirs.insert(normalized);
        }
    }

    pub fn to_report(&self) -> RulesReport {
        let mut excluded_dirs: Vec<String> =
            self.excluded_dirs.iter().map(|p| p.raw.clone()).collect();
        excluded_dirs.sort();

        let mut excluded_extensions: Vec<String> =
            self.excluded_extensions.iter().cloned().collect();
        excluded_extensions.sort();

        let mut always_include_dirs: Vec<String> =
            self.always_include_dirs.iter().cloned().collect();
        always_include_dirs.sort();

        RulesReport {
            source_file: self.source_file.as_ref().map(|p| p.display().to_string()),
            loaded_rules: self.loaded_rules.clone(),
            excluded_dirs,
            excluded_files: self.excluded_files.iter().map(|p| p.raw.clone()).collect(),
            excluded_extensions,
            always_include_files: self
                .always_include_files
                .iter()
                .map(|p| p.raw.clone())
                .collect(),
            always_include_dirs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_rule_strips_comments_and_blank_lines() {
        assert_eq!(clean_rule_line("  "), "");
        assert_eq!(clean_rule_line("# comment"), "");
        assert_eq!(clean_rule_line("*.log # trailing comment"), "*.log");
        assert_eq!(clean_rule_line("hash#file.txt"), "hash#file.txt");
    }

    #[test]
    fn normalize_rule_value_lowercases_and_flips_separators() {
        assert_eq!(
            normalize_rule_value("src\\Utils\\Helper.py"),
            "src/utils/helper.py"
        );
    }

    #[test]
    fn to_report_sorts_the_documented_fields_and_preserves_insertion_order_elsewhere() {
        let mut rules = ExportIgnoreRules::default();
        rules.add_dir_rule("zeta");
        rules.add_dir_rule("alpha");
        rules.add_file_rule("z.txt");
        rules.add_file_rule("a.txt");
        rules.add_extension_rule(".ZIP");
        rules.add_extension_rule(".bin");
        rules.add_always_include_file("z.keep");
        rules.add_always_include_file("a.keep");
        rules.add_always_include_dir("zeta_dir");
        rules.add_always_include_dir("alpha_dir");

        let report = rules.to_report();
        assert_eq!(
            report.excluded_dirs,
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert_eq!(
            report.excluded_files,
            vec!["z.txt".to_string(), "a.txt".to_string()]
        );
        assert_eq!(
            report.excluded_extensions,
            vec!["bin".to_string(), "zip".to_string()]
        );
        assert_eq!(
            report.always_include_files,
            vec!["z.keep".to_string(), "a.keep".to_string()]
        );
        assert_eq!(
            report.always_include_dirs,
            vec!["alpha_dir".to_string(), "zeta_dir".to_string()]
        );
    }
}
