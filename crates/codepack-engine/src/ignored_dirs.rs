//! `ignored_dir_names_for()`: the casefolded ignored-directory-name set legacy computed
//! as `Config.effective_ignored_dirs()` (base defaults ∪ `config.extra_ignored_dirs`,
//! both casefolded) — a method `codepack_core::config::Config` never got a Rust
//! equivalent of, since `codepack-scanner`'s own `build_export_plan` recomputes an
//! equivalent set internally for its own walk. This crate needs its *own* copy of that
//! same set to hand to `codepack_diff::resolve_diff_selection`/`snapshot_project`,
//! which never compute one themselves by design (see `codepack-diff`'s module doc,
//! `docs/decisions/open-questions.md` Q7).

use std::collections::HashSet;
use std::path::Path;

use codepack_core::config::Config;

pub(crate) fn ignored_dir_names_for(source_root: &Path, config: &Config) -> HashSet<String> {
    let mut names: HashSet<String> = codepack_scanner::IGNORED_DIR_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();

    for dir in &config.extra_ignored_dirs {
        let normalized = dir.trim().to_lowercase();
        if !normalized.is_empty() {
            names.insert(normalized);
        }
    }

    let stacks = codepack_scanner::detect_stacks(source_root);
    for dir in codepack_scanner::merged_extra_ignored_dirs(&stacks) {
        let normalized = dir.trim().to_lowercase();
        if !normalized.is_empty() {
            names.insert(normalized);
        }
    }

    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_base_ignored_dir_names_casefolded() {
        let dir = tempfile::tempdir().unwrap();
        let names = ignored_dir_names_for(dir.path(), &Config::default());
        assert!(names.contains("node_modules"));
        assert!(names.contains(".git"));
    }

    #[test]
    fn includes_config_extra_ignored_dirs_casefolded() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            extra_ignored_dirs: vec!["Vendor_Lib".to_string(), "  ".to_string()],
            ..Config::default()
        };
        let names = ignored_dir_names_for(dir.path(), &config);
        assert!(names.contains("vendor_lib"));
        assert!(!names.contains("  "));
        assert!(!names.contains(""));
    }

    #[test]
    fn includes_stack_detected_extra_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let names = ignored_dir_names_for(dir.path(), &Config::default());
        assert!(names.contains("target"));
    }
}
