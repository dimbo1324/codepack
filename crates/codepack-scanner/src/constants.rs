//! Constant sets ported verbatim from legacy `constants.py`
//! (`.ai/project/12-domain-rules.md`: changing a set is a separate decision, never a
//! refactoring side effect). Values, casing, and membership are copied as-is; only the
//! lookup mechanism (a lazily built `HashSet`) is new.

use std::collections::HashSet;
use std::sync::LazyLock;

// Re-exported, not redefined: the three classification sets moved to `codepack-core`
// when Q7 was closed. Keeping the names reachable here preserves every existing
// `codepack_scanner::TEXT_EXTENSIONS` import.
pub use codepack_core::{BINARY_EXTENSIONS, TEXT_EXTENSIONS, TEXT_FILENAMES_WITHOUT_EXTENSION};

/// Directory basenames pruned during the walk, matched case-insensitively. 18 entries.
pub const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    "coverage",
    ".coverage",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
];

static IGNORED_DIR_NAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| IGNORED_DIR_NAMES.iter().copied().collect());

/// `name` must already be lowercased by the caller.
pub(crate) fn is_base_ignored_dir_name(name: &str) -> bool {
    IGNORED_DIR_NAMES_SET.contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sizes_match_legacy_counts() {
        assert_eq!(IGNORED_DIR_NAMES.len(), 18);
    }

    #[test]
    fn no_duplicate_entries() {
        assert_eq!(IGNORED_DIR_NAMES_SET.len(), IGNORED_DIR_NAMES.len());
    }

    #[test]
    fn spot_check_known_members() {
        assert!(is_base_ignored_dir_name("node_modules"));
        assert!(is_base_ignored_dir_name(".venv"));
        assert!(!is_base_ignored_dir_name("src"));
    }
}
