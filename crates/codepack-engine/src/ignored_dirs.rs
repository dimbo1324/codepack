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

/// Legacy's `extra_ignored` variable (`(effective_ignored_dirs | stack_dirs) -
/// IGNORED_DIR_NAMES`): the same union [`ignored_dir_names_for`] computes, minus the
/// base `codepack_scanner::IGNORED_DIR_NAMES` set — the "what did the user/stack add on
/// top of the defaults" subset needed for logging and for
/// `ManifestInput.extra_ignored_dirs`/`IndexInput.extra_ignored_dirs`. Sorted for
/// determinism: callers need a stable, orderable list, even though `write_manifest`'s
/// own `ignored_dirs_payload` re-sorts (into a `BTreeSet`) internally anyway.
///
/// Disclosed, temporary `dead_code` allowance: this pass (S9's Group R+A) adds this
/// helper for [`crate::manifest::write_manifest_and_index`]'s `extra_ignored_dirs`
/// parameter and for [`crate::structure::write_structure_report`]/
/// [`crate::git_report::write_git_report`]'s equivalent inputs, but the top-level
/// orchestrator that actually computes this value once per run and threads it into all
/// three call sites is Group Z's job (`task-checklist.md`), not yet written. Remove
/// this attribute once that wiring lands.
#[allow(dead_code)]
pub(crate) fn extra_ignored_display(source_root: &Path, config: &Config) -> Vec<String> {
    let base: HashSet<String> = codepack_scanner::IGNORED_DIR_NAMES
        .iter()
        .map(|name| name.to_lowercase())
        .collect();
    let mut extras: Vec<String> = ignored_dir_names_for(source_root, config)
        .into_iter()
        .filter(|name| !base.contains(name))
        .collect();
    extras.sort();
    extras
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

    #[test]
    fn extra_ignored_display_excludes_the_base_set() {
        let dir = tempfile::tempdir().unwrap();
        let extras = extra_ignored_display(dir.path(), &Config::default());
        assert!(!extras.iter().any(|name| name == "node_modules"));
        assert!(!extras.iter().any(|name| name == ".git"));
    }

    #[test]
    fn extra_ignored_display_includes_config_and_stack_extras_sorted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let config = Config {
            extra_ignored_dirs: vec!["zzz_custom".to_string()],
            ..Config::default()
        };
        let extras = extra_ignored_display(dir.path(), &config);
        assert!(extras.contains(&"target".to_string()));
        assert!(extras.contains(&"zzz_custom".to_string()));
        let mut sorted = extras.clone();
        sorted.sort();
        assert_eq!(extras, sorted);
    }
}
